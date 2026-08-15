//! # QGC 工作线程（接收 / 发送 / 模拟飞控）
//!
//! 三个线程协作完成与飞控的 MAVLink 通信：
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │ 接收线程：UDP 遥测帧 → 遥测快照（10Hz WS 广播）        │
//! │          任务协议消息（REQUEST_INT/ACK/COUNT/ITEM）   │
//! │          命令回执（COMMAND_ACK）→ WS 事件              │
//! ├──────────────────────────────────────────────────────┤
//! │ 发送线程：心跳（1Hz）+ 数据流请求（5s）               │
//! │          命令/模式（outbound 通道）                   │
//! │          任务状态机（上传条目/下载请求/超时复位）      │
//! ├──────────────────────────────────────────────────────┤
//! │ 模拟器线程（[Udp] Mock=true 时）：真实 MAVLink v2 帧   │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! 上行（飞控 → 地面站）与下行（地面站 → 飞控）各自使用独立的 UDP 套接字：
//! 上行套接字绑定 `LocalPort`（接收遥测 + 学习对端地址），下行套接字为临时端口
//! （发送目标 = 学习对端或配置回退地址），互不阻塞。

use crate::common::ws::{serialize, EventPayload};
use crate::qgc::config;
use crate::qgc::mavlink::{self, FrameExtractor};
use crate::qgc::mission::MissionState;
use crate::qgc::models::QgcMissionItem;
use crate::qgc::simulator;
use crate::qgc::state::{self, Outbound};
use crate::qgc::udp::UdpLink;
use crate::qgc::{CommandAckPayload, MissionProgress, QgcEvent};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// 遥测广播节流间隔（100ms = 10Hz，与 `[Gcs] TelemetryHz` 默认一致）
const TELEMETRY_EMIT_INTERVAL: Duration = Duration::from_millis(100);
/// 连接超时（距上次心跳超过该时间判定为断开）
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);
/// 数据流请求间隔（5 秒，请求飞控以 10Hz 发送遥测）
const DATA_STREAM_INTERVAL: Duration = Duration::from_secs(5);

/// 启动全部 QGC 工作线程
///
/// # 参数
/// - `stop`：停止信号（工作线程轮询此标志退出）
/// - `tx`：广播通道发送端（WebSocket 推送）
///
/// # 返回值
/// 所有工作线程的句柄列表（由 `service::stop_service` 统一 join）；
/// UDP 绑定失败时返回错误信息。
///
/// # 线程数量
/// - 模拟模式：3（接收 + 发送 + 模拟飞控）
/// - 真实模式：2（接收 + 发送）
pub fn start_all(
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<EventPayload>,
) -> Result<Vec<thread::JoinHandle<()>>, String> {
    let (mock, local_port, target_ip, target_port, gcs_sysid, gcs_compid, heartbeat_ms, telemetry_hz) = config::udp_params();
    info!(
        "[qgc] 启动服务：mock={} local_port={} target={}:{} gcs={}/{} heartbeat={}ms telemetry={}Hz",
        mock, local_port, target_ip, target_port, gcs_sysid, gcs_compid, heartbeat_ms, telemetry_hz
    );

    // 上行套接字（绑定本地端口，接收遥测 + 学习对端）
    let link = Arc::new(UdpLink::create("0.0.0.0", local_port, &target_ip, target_port)?);

    // 下行命令通道（HTTP handler 写入，发送线程消费）
    let (out_tx, out_rx) = std::sync::mpsc::channel::<Outbound>();
    state::set_outbound(out_tx);

    // 复位上次会话残留状态
    state::reset_telemetry();
    state::reset_mission();

    let mut handles = Vec::new();
    {
        let (stop2, tx2, link2) = (stop.clone(), tx.clone(), link.clone());
        handles.push(thread::spawn(move || run_receiver(stop2, tx2, link2, telemetry_hz)));
    }
    {
        let (stop3, tx3, link3) = (stop.clone(), tx.clone(), link.clone());
        handles.push(thread::spawn(move || run_sender(stop3, tx3, link3, out_rx, gcs_sysid, gcs_compid, heartbeat_ms)));
    }
    if mock {
        let (stop4, mission) = (stop.clone(), state::mission());
        handles.push(thread::spawn(move || simulator::run_simulator(local_port, stop4, mission)));
    }

    info!("[qgc] 已启动 {} 个工作线程", handles.len());
    Ok(handles)
}

// ════════════════════════════════════════════════════════════
//  接收线程（飞控 → 地面站）
// ════════════════════════════════════════════════════════════

/// 接收线程：解析 MAVLink 帧，更新遥测快照与任务状态，10Hz 广播
fn run_receiver(
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<EventPayload>,
    link: Arc<UdpLink>,
    _telemetry_hz: u16,
) {
    let mut extractor = FrameExtractor::new();
    let mut buf = vec![0u8; 4096];
    let mut last_emit = Instant::now();
    // 帧速率统计（2 秒滑动窗口）
    let mut rate_window_start = Instant::now();
    let mut rate_count: u32 = 0;
    // 心跳跟踪（连接超时判定）
    let mut last_heartbeat = Instant::now();

    info!("[qgc] 接收线程启动，监听 {}", link.local_addr());
    while !stop.load(Ordering::Relaxed) {
        match link.recv_from(&mut buf) {
            Ok((len, src)) => {
                // 学习对端：命令发送指向最近活跃的飞控（模拟器 / 真实硬件自动适配）
                link.learn(src);
                let frames = extractor.feed(&buf[..len]);
                if frames.is_empty() {
                    continue;
                }
                // 更新遥测快照（先读副本，批量写入一次）
                let mut t = state::telemetry().read().unwrap_or_else(|e| e.into_inner()).clone();
                for f in &frames {
                    rate_count += 1;
                    match f.msgid {
                        mavlink::msg::HEARTBEAT => {
                            let hb = mavlink::decode_heartbeat(&f.payload);
                            t.connected = true;
                            t.sysid = f.sysid;
                            t.armed = hb.base_mode & mavlink::consts::MAV_MODE_FLAG_SAFETY_ARMED != 0;
                            t.mode = mavlink::mode_name(hb.custom_mode);
                            t.vehicle_type = hb.vehicle_type;
                            last_heartbeat = Instant::now();
                        }
                        mavlink::msg::SYS_STATUS => {
                            let s = mavlink::decode_sys_status(&f.payload);
                            t.cpu_load = s.load as f32 / 10.0;
                            t.voltage = s.voltage_battery as f32 / 1000.0;
                            t.current = s.current_battery as f32 / 100.0;
                            t.battery_remaining = s.battery_remaining;
                        }
                        mavlink::msg::BATTERY_STATUS => {
                            let b = mavlink::decode_battery_status(&f.payload);
                            t.voltage = b.voltage as f32 / 1000.0;
                            t.current = b.current_battery as f32 / 100.0;
                            if b.battery_remaining >= 0 {
                                t.battery_remaining = b.battery_remaining;
                            }
                            t.battery_consumed_mah = b.current_consumed.max(0) as f32;
                        }
                        mavlink::msg::GPS_RAW_INT => {
                            let g = mavlink::decode_gps_raw_int(&f.payload);
                            t.gps_fix_type = g.fix_type;
                            t.gps_eph = g.eph as f32 / 1000.0;
                            t.satellites_visible = g.satellites_visible;
                            if g.lat != 0 || g.lon != 0 {
                                t.lat = g.lat as f64 / 1e7;
                                t.lon = g.lon as f64 / 1e7;
                            }
                            if g.alt != 0 {
                                t.altitude = g.alt as f32 / 1000.0;
                            }
                        }
                        mavlink::msg::ATTITUDE => {
                            let a = mavlink::decode_attitude(&f.payload);
                            t.roll = a.roll.to_degrees();
                            t.pitch = a.pitch.to_degrees();
                            t.heading = a.yaw.to_degrees().rem_euclid(360.0);
                            t.roll_rate = a.rollspeed.to_degrees();
                            t.pitch_rate = a.pitchspeed.to_degrees();
                            t.yaw_rate = a.yawspeed.to_degrees();
                        }
                        mavlink::msg::GLOBAL_POSITION_INT => {
                            let g = mavlink::decode_global_position_int(&f.payload);
                            if g.lat != 0 || g.lon != 0 {
                                t.lat = g.lat as f64 / 1e7;
                                t.lon = g.lon as f64 / 1e7;
                            }
                            t.altitude = g.alt as f32 / 1000.0;
                            t.relative_alt = g.relative_alt as f32 / 1000.0;
                            t.groundspeed = ((g.vx as f32).powi(2) + (g.vy as f32).powi(2)).sqrt() / 100.0;
                        }
                        mavlink::msg::VFR_HUD => {
                            let v = mavlink::decode_vfr_hud(&f.payload);
                            t.airspeed = v.airspeed;
                            t.groundspeed = v.groundspeed;
                            t.climb = v.climb;
                            t.throttle = v.throttle as f32;
                            if v.heading >= 0 {
                                t.heading = v.heading as f32;
                            }
                            t.altitude = v.alt;
                        }
                        // ── 任务协议（仅接受常规任务 MISSION_TYPE_MISSION） ──
                        mavlink::msg::MISSION_REQUEST_INT => {
                            let r = mavlink::decode_mission_request_int(&f.payload);
                            if r.mission_type != mavlink::consts::MISSION_TYPE_MISSION {
                                continue;
                            }
                            let ms = state::mission();
                            let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                            if m.state == "uploading" {
                                m.last_request_seq = Some(r.seq);
                                emit_mission_progress(&tx, &m);
                            }
                        }
                        mavlink::msg::MISSION_ACK => {
                            let a = mavlink::decode_mission_ack(&f.payload);
                            if a.mission_type != mavlink::consts::MISSION_TYPE_MISSION {
                                continue;
                            }
                            let ms = state::mission();
                            let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                            let result = if a.result == 0 { "ok" } else { "failed" };
                            // 清除成功后清空本地航点列表（上传成功则保留已上传内容）
                            if m.state == "clearing" && a.result == 0 {
                                m.items.clear();
                            }
                            m.finish(result);
                            emit_mission_progress(&tx, &m);
                        }
                        mavlink::msg::MISSION_COUNT => {
                            let c = mavlink::decode_mission_count(&f.payload);
                            if c.mission_type != mavlink::consts::MISSION_TYPE_MISSION {
                                continue;
                            }
                            let ms = state::mission();
                            let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                            if m.state == "downloading" {
                                m.total = c.count;
                                m.items.clear();
                                m.received = 0;
                                if c.count == 0 {
                                    m.finish("ok");
                                }
                                emit_mission_progress(&tx, &m);
                            }
                        }
                        mavlink::msg::MISSION_ITEM_INT => {
                            let it = mavlink::decode_mission_item_int(&f.payload);
                            if it.mission_type != mavlink::consts::MISSION_TYPE_MISSION {
                                continue;
                            }
                            let ms = state::mission();
                            let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                            if m.state == "downloading" && it.seq == m.received {
                                m.items.push(QgcMissionItem {
                                    seq: it.seq,
                                    command: it.command,
                                    lat: it.x as f64 / 1e7,
                                    lon: it.y as f64 / 1e7,
                                    altitude: it.z,
                                });
                                m.received += 1;
                                if m.received >= m.total {
                                    m.finish("ok");
                                }
                                emit_mission_progress(&tx, &m);
                            }
                        }
                        mavlink::msg::COMMAND_ACK => {
                            let a = mavlink::decode_command_ack(&f.payload);
                            let name = match a.result {
                                0 => "ACCEPTED",
                                1 => "TEMPORARILY_REJECTED",
                                2 => "DENIED",
                                3 => "UNSUPPORTED",
                                4 => "FAILED",
                                5 => "IN_PROGRESS",
                                6 => "INVALID_SEQUENCE",
                                7 => "UNSUPPORTED_FRAME",
                                _ => "UNKNOWN",
                            };
                            if let Ok(json) = serialize(&QgcEvent::CommandAck(CommandAckPayload {
                                command: a.command,
                                result: a.result,
                                result_name: name.to_string(),
                            })) {
                                let _ = tx.send(json);
                            }
                        }
                        _ => {
                            debug!("[qgc] 未处理消息 {}（sysid={} compid={}）", f.msgid, f.sysid, f.compid);
                        }
                    }
                }
                // 帧速率统计（2 秒窗口）
                if rate_window_start.elapsed() >= Duration::from_secs(2) {
                    t.packet_rate = rate_count as f32 / 2.0;
                    rate_count = 0;
                    rate_window_start = Instant::now();
                }
                // 连接超时判定
                let since_hb = last_heartbeat.elapsed().as_millis() as u64;
                t.last_heartbeat_ms = since_hb;
                if since_hb > CONNECTION_TIMEOUT.as_millis() as u64 {
                    t.connected = false;
                }
                *state::telemetry().write().unwrap_or_else(|e| e.into_inner()) = t;
                // 10Hz 节流广播
                if since_hb <= CONNECTION_TIMEOUT.as_millis() as u64 && last_emit.elapsed() >= TELEMETRY_EMIT_INTERVAL {
                    last_emit = Instant::now();
                    let snapshot = state::telemetry().read().unwrap_or_else(|e| e.into_inner()).clone();
                    if let Ok(json) = serialize(&QgcEvent::Telemetry(snapshot)) {
                        let _ = tx.send(json);
                    }
                }
                debug!("[qgc] 收到 {} 字节（{} 帧），来自 {}", len, frames.len(), src);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                // 读超时：轮询停止信号；同时刷新连接状态（断连时广播一次）
                let since_hb = last_heartbeat.elapsed().as_millis() as u64;
                if since_hb > CONNECTION_TIMEOUT.as_millis() as u64 {
                    let tel = state::telemetry();
                    let mut t = tel.write().unwrap_or_else(|e| e.into_inner());
                    if t.connected {
                        t.connected = false;
                        t.last_heartbeat_ms = since_hb;
                        let snapshot = t.clone();
                        drop(t);
                        if let Ok(json) = serialize(&QgcEvent::Telemetry(snapshot)) {
                            let _ = tx.send(json);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("[qgc] 接收错误: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    info!("[qgc] 接收线程已停止");
}

// ════════════════════════════════════════════════════════════
//  发送线程（地面站 → 飞控）
// ════════════════════════════════════════════════════════════

/// 发送线程：心跳 / 数据流请求 / 下行命令 / 任务状态机
fn run_sender(
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<EventPayload>,
    link: Arc<UdpLink>,
    out_rx: std::sync::mpsc::Receiver<Outbound>,
    gcs_sysid: u8,
    gcs_compid: u8,
    heartbeat_ms: u64,
) {
    // 独立发送套接字（临时端口，避免与接收线程争用）
    let sock = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(s) => s,
        Err(e) => {
            error!("[qgc] 创建发送套接字失败: {}", e);
            return;
        }
    };
    let heartbeat_interval = Duration::from_millis(heartbeat_ms.max(100));
    let mut last_heartbeat = Instant::now() - heartbeat_interval;
    let mut last_data_stream = Instant::now();

    info!("[qgc] 发送线程启动（sysid={} compid={}）", gcs_sysid, gcs_compid);
    while !stop.load(Ordering::Relaxed) {
        // 1. 消费下行命令通道
        while let Ok(msg) = out_rx.try_recv() {
            match msg {
                Outbound::Frame(frame) => {
                    let _ = sock.send_to(&frame, link.send_target());
                }
                Outbound::MissionUpload(items) => {
                    // 首页（seq 0）自动补上当前起飞点，真实飞控（ArduPilot）要求第 0 条为首页
                    let home = state::telemetry().read().unwrap_or_else(|e| e.into_inner()).clone();
                    let mut all: Vec<QgcMissionItem> = Vec::with_capacity(items.len() + 1);
                    all.push(QgcMissionItem {
                        seq: 0,
                        command: mavlink::cmd::NAV_WAYPOINT,
                        lat: home.lat,
                        lon: home.lon,
                        altitude: 0.0,
                    });
                    for (i, mut it) in items.into_iter().enumerate() {
                        it.seq = (i + 1) as u16;
                        it.command = mavlink::cmd::NAV_WAYPOINT;
                        all.push(it);
                    }
                    let count = all.len() as u16;
                    let ms = state::mission();
                    let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                    m.items = all;
                    m.begin("uploading", count);
                    let frame = mavlink::encode_mission_count(gcs_sysid, gcs_compid, state::next_seq(), 1, 1, count);
                    drop(m);
                    emit_mission_progress(&tx, &ms.lock().unwrap_or_else(|e| e.into_inner()));
                    let _ = sock.send_to(&frame, link.send_target());
                    info!("[qgc] 任务上传开始：{} 条航点", count - 1);
                }
                Outbound::MissionDownload => {
                    let ms = state::mission();
                    let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                    m.begin("downloading", 0);
                    let frame = mavlink::encode_mission_request_list(gcs_sysid, gcs_compid, state::next_seq(), 1, 1);
                    drop(m);
                    emit_mission_progress(&tx, &ms.lock().unwrap_or_else(|e| e.into_inner()));
                    let _ = sock.send_to(&frame, link.send_target());
                    info!("[qgc] 任务下载开始");
                }
                Outbound::MissionClear => {
                    let ms = state::mission();
                    let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
                    m.begin("clearing", 0);
                    let frame = mavlink::encode_mission_clear_all(gcs_sysid, gcs_compid, state::next_seq(), 1, 1);
                    drop(m);
                    emit_mission_progress(&tx, &ms.lock().unwrap_or_else(|e| e.into_inner()));
                    let _ = sock.send_to(&frame, link.send_target());
                    info!("[qgc] 任务清除开始");
                }
            }
        }

        // 2. 任务状态机推进（发送线程每 100ms 轮询）
        {
            let ms = state::mission();
            let mut m = ms.lock().unwrap_or_else(|e| e.into_inner());
            match m.state.as_str() {
                "uploading" => {
                    // 飞控请求序号 → 发送对应条目
                    if let Some(n) = m.last_request_seq.take() {
                        if let Some(item) = m.items.get(n as usize) {
                            let frame = mavlink::encode_mission_item_int(
                                gcs_sysid, gcs_compid, state::next_seq(), 1, 1, n, item.lat, item.lon, item.altitude,
                            );
                            let target = link.send_target();
                            let _ = sock.send_to(&frame, target);
                            debug!("[qgc] 上传条目 seq={}", n);
                        } else {
                            // 飞控请求了不存在的条目：直接结束（避免死循环）
                            m.finish("invalid_seq");
                            emit_mission_progress(&tx, &m);
                        }
                    }
                }
                "downloading" => {
                    // 已收到 MISSION_COUNT 后逐个请求条目
                    if m.received < m.total {
                        let frame = mavlink::encode_mission_request_int(
                            gcs_sysid, gcs_compid, state::next_seq(), 1, 1, m.received,
                        );
                        let target = link.send_target();
                        let _ = sock.send_to(&frame, target);
                    }
                }
                "clearing" => {
                    // 等待飞控 MISSION_ACK，无需发送
                }
                _ => {}
            }
            if m.check_timeout() {
                warn!("[qgc] 任务 {} 超时", m.state);
                emit_mission_progress(&tx, &m);
            }
        }

        // 3. 心跳（1Hz 默认）
        if last_heartbeat.elapsed() >= heartbeat_interval {
            last_heartbeat = Instant::now();
            let frame = mavlink::encode_heartbeat(
                gcs_sysid,
                gcs_compid,
                state::next_seq(),
                mavlink::consts::MAV_TYPE_GCS,
                mavlink::consts::MAV_AUTOPILOT_INVALID,
                mavlink::consts::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
                0,
                mavlink::consts::MAV_STATE_ACTIVE,
            );
            let _ = sock.send_to(&frame, link.send_target());
        }
        // 4. 请求数据流（5 秒一次，请求飞控以 10Hz 发送遥测）
        if last_data_stream.elapsed() >= DATA_STREAM_INTERVAL {
            last_data_stream = Instant::now();
            let frame = mavlink::encode_request_data_stream(gcs_sysid, gcs_compid, state::next_seq(), 1, 1, 10);
            let _ = sock.send_to(&frame, link.send_target());
        }

        thread::sleep(Duration::from_millis(100));
    }
    info!("[qgc] 发送线程已停止");
}

// ════════════════════════════════════════════════════════════
//  事件发射
// ════════════════════════════════════════════════════════════

/// 发射任务进度事件（预序列化一次后广播）
fn emit_mission_progress(tx: &broadcast::Sender<EventPayload>, m: &MissionState) {
    let progress = MissionProgress {
        state: m.state.clone(),
        total: m.total,
        received: m.received,
        result: m.result.clone(),
        current_seq: m.current_seq,
    };
    if let Ok(json) = serialize(&QgcEvent::MissionProgress(progress)) {
        let _ = tx.send(json);
    }
}
