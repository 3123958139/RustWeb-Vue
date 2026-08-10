//! # UDP 工作线程
//!
//! 主/备双链路帧提取（主备切换）、6 路单连接（收发）、进程内模拟数据源。
//! 从 demo-test3-ftj 的 backend/process.rs 移植，事件发射由 Tauri emit
//! 替换为 `broadcast::Sender<Ftj1cEvent>` 广播（WebSocket 推送）。
//!
//! ## 模拟模式
//!
//! `config-ftj1c.ini` 的 `[Udp] Mock = true`（默认）时使用进程内数据源，无需硬件：
//! 按 200ms 周期向各链路生成 `EB 90 5B` 帧；IP11（主链）在 5~10 秒窗口暂停，
//! 可验证主备切换。`Mock = false` 时使用真实 UDP 套接字（组播收发）。
//!
//! ## 线程模型
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    进程内线程池                          │
//! ├─────────────────────────────────────────────────────────┤
//! │ [模拟/真实] 主链接收线程 → QuadFrame[0..3]              │
//! │ [模拟/真实] 备链接收线程 → QuadFrame[0..3]              │
//! │ [模拟/真实] 单路连接线程 ×6 → 发送/接收                 │
//! │ [始终]       串口发送线程 ×3 → 串口设备                  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 事件发射节流
//!
//! 使用 `Throttle` 结构控制事件发射频率（50ms = 20 FPS），
//! 避免高频数据导致前端 WebSocket 过载。

use crate::common::frame_extractor::FrameExtractor;
use crate::common::utils::{format_hex, parse_hex, parse_ip_to_port};
use crate::ftj1c::com::{make_transform, ComControl, ComFTJ1CBase};
use crate::ftj1c::config;
use crate::ftj1c::quad_frame::{QuadFrame, FRAME_LEN, SRC_PRIMARY, SRC_SECONDARY};
use crate::ftj1c::udp::{UdpControl, UdpMode};
use crate::ftj1c::{Ftj1cEvent, UdpDataPayload};
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

// ─── 前端发射节流（50ms = 20 FPS） ───
/// 事件发射间隔（50 毫秒，对应 20 FPS）
///
/// # 说明
/// 后端全速处理帧数据并写入 `QuadFrame`，仅定期推送最新快照到前端，
/// 避免高频数据导致 WebSocket 过载。
const EMIT_INTERVAL: Duration = Duration::from_millis(50);

/// 事件发射节流器
///
/// # 说明
/// 控制事件发射频率，确保前端接收速率不超过 20 FPS。
/// 在高频数据场景下，超过间隔的事件会被丢弃。
struct Throttle {
    /// 上次发射时间
    last_emit: Instant,
}

impl Throttle {
    /// 创建新的节流器（从当前时间开始计时）
    fn new() -> Self {
        Self {
            last_emit: Instant::now(),
        }
    }

    /// 检查是否可以发射事件
    ///
    /// # 返回值
    /// - `true`: 距离上次发射已超过 `EMIT_INTERVAL`，可以发射
    /// - `false`: 间隔不足，应跳过本次发射
    fn ready(&mut self) -> bool {
        let now = Instant::now();
        if now >= self.last_emit + EMIT_INTERVAL {
            self.last_emit = now;
            true
        } else {
            false
        }
    }
}

// ─── 单帧提取结果（frame_extractor decoder 输出通道） ───

/// 从字节流中提取的单帧数据
///
/// # 说明
/// 由 `FrameExtractor` 的 `decoder` 闭包输出，包含槽位索引、序列号和原始帧数据。
#[derive(Debug, Clone)]
struct ExtractedFrame {
    /// 槽位索引 (0..3)
    slot: usize,
    /// 帧序列号（用于去重）
    seq: u32,
    /// 完整 95 字节帧数据
    data: [u8; FRAME_LEN],
}

// ─── 连接配置（单路 UDP） ───

/// 单路 UDP 连接配置
///
/// # 说明
/// 定义单路 UDP 连接的模式、本地/远程 IP 键名和连接序号。
/// 用于配置文件读取和事件发射时的标识。
#[derive(Clone)]
struct ConnSpec {
    /// UDP 通信模式（发送/接收）
    udp_mode: UdpMode,
    /// 本地 IP 配置键名（如 "IP2"）
    local_key: String,
    /// 远程 IP 配置键名（如 "IP1"）
    remote_key: String,
    /// 连接序号（用于事件发射）
    conn_idx: usize,
}

/// 获取单路 UDP 连接配置列表
///
/// # 返回值
/// 6 个连接配置的向量，对应 6 路单路 UDP 连接
///
/// # 连接映射
/// ```text
/// conn 0: IP2 → IP1 (发送)
/// conn 1: IP4 → IP3 (发送)
/// conn 2: IP6 ← IP5 (接收)
/// conn 3: IP8 ← IP7 (接收)
/// conn 4: IP10 → IP9 (发送)
/// conn 6: IP14 → IP13 (发送)
/// ```
fn single_conn_specs() -> Vec<ConnSpec> {
    vec![
        ConnSpec { udp_mode: UdpMode::Send, local_key: "IP2".into(), remote_key: "IP1".into(), conn_idx: 0 },
        ConnSpec { udp_mode: UdpMode::Send, local_key: "IP4".into(), remote_key: "IP3".into(), conn_idx: 1 },
        ConnSpec { udp_mode: UdpMode::Recv, local_key: "IP6".into(), remote_key: "IP5".into(), conn_idx: 2 },
        ConnSpec { udp_mode: UdpMode::Recv, local_key: "IP8".into(), remote_key: "IP7".into(), conn_idx: 3 },
        ConnSpec { udp_mode: UdpMode::Send, local_key: "IP10".into(), remote_key: "IP9".into(), conn_idx: 4 },
        ConnSpec { udp_mode: UdpMode::Send, local_key: "IP14".into(), remote_key: "IP13".into(), conn_idx: 6 },
    ]
}

// ════════════════════════════════════════════════════════════
//  入口
// ════════════════════════════════════════════════════════════

/// 启动全部工作线程
///
/// # 参数
/// - `qf`: 共享 QuadFrame，用于帧数据交换
/// - `stop`: 停止信号，工作线程轮询此标志退出
/// - `tx`: 广播通道发送端，用于向 WebSocket 客户端推送事件
///
/// # 返回值
/// 所有工作线程的句柄列表，由 `service::stop_service` 统一 `join`
///
/// # 线程数量
/// - 模拟模式：2（主备链）+ 6（单路）+ 3（串口）= 11 个线程
/// - 真实模式：2（主备链）+ 6（单路）+ 3（串口）= 11 个线程
pub fn start_all(
    qf: Arc<QuadFrame>,
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Vec<thread::JoinHandle<()>> {
    let mock = config::global()
        .map(|c| c.get_or("Udp", "Mock", "true").eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    let mut handles = Vec::new();

    if mock {
        info!("[ftj1c] 模拟模式：使用进程内数据源（config-ftj1c.ini [Udp] Mock=true）");
        {
            let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
            handles.push(thread::spawn(move || run_mock_primary(qf, stop, tx)));
        }
        {
            let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
            handles.push(thread::spawn(move || run_mock_secondary(qf, stop, tx)));
        }
        for spec in single_conn_specs() {
            let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
            handles.push(thread::spawn(move || run_mock_single(&spec, qf, stop, tx)));
        }
    } else {
        info!("[ftj1c] 真实模式：使用 UDP 套接字（config-ftj1c.ini [IP] 配置）");
        {
            let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
            handles.push(thread::spawn(move || run_primary_secondary(qf, stop, tx)));
        }
        for spec in single_conn_specs() {
            let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
            handles.push(thread::spawn(move || run_single_connection(qf, &spec, stop, tx)));
        }
    }

    // 串口发送（3 路，与 demo-test3-ftj 一致：始终启动，串口不可用时报错退出线程）
    {
        let (qf, stop) = (qf.clone(), stop.clone());
        handles.push(thread::spawn(move || run_coms(qf, stop)));
    }

    info!("[ftj1c] 已启动 {} 个工作线程", handles.len());
    handles
}

// ════════════════════════════════════════════════════════════
//  frame_extractor 配置
// ════════════════════════════════════════════════════════════

/// 帧同步头：`EB 90 5B`
const HEADER: [u8; 3] = [0xEB, 0x90, 0x5B];

/// 帧校验闭包：验证帧的有效性
///
/// # 校验规则
/// 1. 帧长必须为 95 字节
/// 2. 帧头必须为 `EB 90 5B`
/// 3. 槽位字节必须为 1~4
/// 4. 前 93 字节累加和必须等于尾部校验和（LE u16）
///
/// # 参数
/// - `frame`: 待校验的帧数据
///
/// # 返回值
/// `true` 表示帧有效，`false` 表示无效
fn frame_validator(frame: &[u8]) -> bool {
    if frame.len() < FRAME_LEN {
        return false;
    }
    // 检查帧头
    if frame[0] != HEADER[0] || frame[1] != HEADER[1] || frame[2] != HEADER[2] {
        return false;
    }
    // 检查槽位字节（1~4）
    if !(1..=4).contains(&frame[3]) {
        return false;
    }
    // 计算并验证校验和
    let sum: u16 = frame[0..93].iter().map(|&b| b as u16).sum();
    let checksum = u16::from_le_bytes([frame[93], frame[94]]);
    sum == checksum
}

/// 创建帧解码闭包
///
/// # 参数
/// - `result`: 共享结果容器，解码后的帧数据写入此处
///
/// # 返回值
/// 解码闭包，符合 `FrameExtractor` 的 `decoder` 接口
///
/// # 说明
/// 闭包捕获 `result` 的克隆（`Arc`），将提取的帧数据写入共享容器，
/// 供工作线程读取。
fn make_decoder(
    result: Arc<Mutex<Option<ExtractedFrame>>>,
) -> impl Fn(&[u8]) -> bool + Send + Sync + 'static {
    move |frame: &[u8]| {
        if frame.len() < FRAME_LEN {
            return false;
        }
        // 从帧头提取槽位索引（frame[3] - 1）
        let slot = (frame[3] - 1) as usize;
        // 从帧头提取序列号（小端 u32）
        let seq = u32::from_le_bytes(frame[4..8].try_into().unwrap());
        // 复制完整帧数据
        let mut data = [0u8; FRAME_LEN];
        data.copy_from_slice(&frame[..FRAME_LEN]);
        // 写入共享结果容器
        *result.lock().unwrap_or_else(|e| e.into_inner()) = Some(ExtractedFrame { slot, seq, data });
        true
    }
}

/// 创建帧提取器实例
///
/// # 参数
/// - `result`: 共享结果容器
///
/// # 返回值
/// 配置好帧协议参数的 `FrameExtractor` 实例
fn new_extractor(result: Arc<Mutex<Option<ExtractedFrame>>>) -> FrameExtractor {
    let ext = result.clone();
    FrameExtractor::new(
        HEADER.to_vec(),
        FRAME_LEN,
        Box::new(frame_validator),
        Box::new(make_decoder(ext)),
    )
}

// ════════════════════════════════════════════════════════════
//  事件发射
// ════════════════════════════════════════════════════════════

/// 发射 UDP 数据事件到广播通道
///
/// # 参数
/// - `tx`: 广播通道发送端
/// - `conn_idx`: 连接序号 (0..7)
/// - `slot`: 槽位索引 (0..3)
/// - `source`: 来源标识（"P"=主链, "S"=备链, "-"=单路）
/// - `local_ip`: 本地 IP 键名
/// - `local_port`: 本地端口
/// - `remote_ip`: 远程 IP 键名
/// - `remote_port`: 远程端口
/// - `raw_hex`: 原始数据十六进制
/// - `ext_hex`: 提取/发送数据十六进制
///
/// # 说明
/// 事件通过 `broadcast::Sender` 发送，所有订阅的 WebSocket 客户端都会收到。
/// 如果通道满，最旧的事件会被丢弃。
#[allow(clippy::too_many_arguments)]
fn emit(
    tx: &broadcast::Sender<crate::common::ws::EventPayload>,
    conn_idx: usize,
    slot: usize,
    source: &str,
    local_ip: &str,
    local_port: u16,
    remote_ip: &str,
    remote_port: u16,
    raw_hex: String,
    ext_hex: String,
) {
    // 预序列化一次后广播（N 个订阅者只克隆 Arc<str> 指针，不重复序列化）
    let event = Ftj1cEvent::UdpData(UdpDataPayload {
        connection_index: conn_idx,
        slot_index: slot,
        source: source.to_string(),
        local_ip: local_ip.to_string(),
        local_port,
        remote_ip: remote_ip.to_string(),
        remote_port,
        raw_hex,
        ext_hex,
    });
    if let Ok(json) = crate::common::ws::serialize(&event) {
        let _ = tx.send(json);
    }
}

// ════════════════════════════════════════════════════════════
//  真实模式：主副切换双链路
// ════════════════════════════════════════════════════════════

/// 真实模式：主备双链路线程
///
/// # 说明
/// 启动主链（IP11）和备链（IP15）两个接收线程，
/// 实现主备切换和帧数据转发。
///
/// # 线程
/// - 主链接收线程：接收主链 UDP 数据，写入 QuadFrame，转发到目标
/// - 备链接收线程：接收备链 UDP 数据，写入 QuadFrame，转发到目标
fn run_primary_secondary(qf: Arc<QuadFrame>, stop: Arc<AtomicBool>, tx: broadcast::Sender<crate::common::ws::EventPayload>) {
    let cfg = match config::global() {
        Some(c) => c,
        None => {
            error!("[PS] 配置未加载");
            return;
        }
    };

    // 主链 IP11 (src) / IP12 (local)
    let primary_local = cfg.get_or("IP", "ip12", "0.0.0.0");
    let primary_dest = cfg.get_or("IP", "ip11", "226.0.0.80");
    let primary_port: u16 = cfg.get_or("IP", "port11", "8004").parse().unwrap_or(8004);

    // 备链 IP15 (src) / IP16 (local)
    let secondary_local = cfg.get_or("IP", "ip16", "0.0.0.0");
    let secondary_dest = cfg.get_or("IP", "ip15", "226.0.0.82");
    let secondary_port: u16 = cfg.get_or("IP", "port15", "6091").parse().unwrap_or(6091);

    let send_local = "0.0.0.0".to_string();
    let send_dest = cfg.get_or("IP", "ip1", "0.0.0.0");
    let send_primary_port: u16 = cfg.get_or("IP", "port1", "20004").parse().unwrap_or(20004);

    info!(
        "[PS] Primary: {}:{} -> {}, Secondary: {}:{} -> {}, 转发目标: {}:{}",
        primary_local, primary_port, primary_dest, secondary_local, secondary_port, secondary_dest,
        send_dest, send_primary_port
    );

    let udp_send_primary = match UdpControl::create(&send_local, &send_dest, send_primary_port, UdpMode::Send) {
        Ok(u) => u,
        Err(e) => {
            error!("[PS] 创建转发发送套接字失败: {}", e);
            return;
        }
    };
    let udp_send_secondary = match UdpControl::create(&send_local, &send_dest, send_primary_port, UdpMode::Send) {
        Ok(u) => u,
        Err(e) => {
            error!("[PS] 创建转发发送套接字失败: {}", e);
            return;
        }
    };

    // ── 主链接收线程 ──
    {
        let (qf, stop, tx) = (qf.clone(), stop.clone(), tx.clone());
        thread::spawn(move || {
            let udp = match UdpControl::create(&primary_local, &primary_dest, primary_port, UdpMode::Recv) {
                Ok(u) => u,
                Err(e) => {
                    error!("[Primary] 创建 UDP 失败: {}", e);
                    return;
                }
            };

            let result = Arc::new(Mutex::new(None::<ExtractedFrame>));
            let ext = result.clone();
            let mut fe = new_extractor(ext);
            let mut buf = [0u8; 4096];
            // 背压保护：UDP 接收缓冲区设为 1MB，防止突发丢包
            let _ = udp.set_recv_buffer_size(1024 * 1024);
            let mut throttle = Throttle::new();

            while !stop.load(Ordering::Relaxed) {
                match udp.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        fe.feed(&buf[..len]);
                        if let Some(ef) = result.lock().unwrap_or_else(|e| e.into_inner()).take() {
                            let _ = udp_send_primary.send_to(&ef.data);
                            // 始终处理帧（写 QuadFrame 做去重 + 主备切换）
                            let accepted = qf.try_update(ef.slot, SRC_PRIMARY, ef.seq, &ef.data);
                            // 节流发射：仅按 EMIT_INTERVAL 向前端推送
                            if throttle.ready() {
                                emit(
                                    &tx,
                                    5,
                                    ef.slot,
                                    "P",
                                    "",
                                    0,
                                    "",
                                    0,
                                    format!(
                                        "accepted?{}>{}:[{}]{}",
                                        accepted,
                                        "接收",
                                        len,
                                        format_hex(&buf[..len])
                                    ),
                                    format_hex(&ef.data),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("[Primary] recv error: {}", e);
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            info!("[Primary] 线程已停止");
        });
    }

    // ── 备链接收线程 ──
    {
        let (qf, stop, tx) = (qf.clone(), stop.clone(), tx);
        thread::spawn(move || {
            let udp = match UdpControl::create(&secondary_local, &secondary_dest, secondary_port, UdpMode::Recv) {
                Ok(u) => u,
                Err(e) => {
                    error!("[Secondary] 创建 UDP 失败: {}", e);
                    return;
                }
            };

            let result = Arc::new(Mutex::new(None::<ExtractedFrame>));
            let ext = result.clone();
            let mut fe = new_extractor(ext);
            let mut buf = [0u8; 4096];
            let _ = udp.set_recv_buffer_size(1024 * 1024);
            let mut throttle = Throttle::new();

            while !stop.load(Ordering::Relaxed) {
                match udp.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        fe.feed(&buf[..len]);
                        if let Some(ef) = result.lock().unwrap_or_else(|e| e.into_inner()).take() {
                            let _ = udp_send_secondary.send_to(&ef.data);
                            let accepted = qf.try_update(ef.slot, SRC_SECONDARY, ef.seq, &ef.data);
                            if throttle.ready() {
                                emit(
                                    &tx,
                                    7,
                                    ef.slot,
                                    "S",
                                    "",
                                    0,
                                    "",
                                    0,
                                    format!(
                                        "accepted?{}>{}:[{}]{}",
                                        accepted,
                                        "接收",
                                        len,
                                        format_hex(&buf[..len])
                                    ),
                                    format_hex(&ef.data),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("[Secondary] recv error: {}", e);
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            info!("[Secondary] 线程已停止");
        });
    }

    info!("[PS] 主副双链路已启动");
}

// ════════════════════════════════════════════════════════════
//  真实模式：单路 UDP 连接
// ════════════════════════════════════════════════════════════

/// 真实模式：单路 UDP 连接线程
///
/// # 参数
/// - `qf`: 共享 QuadFrame
/// - `spec`: 连接配置（模式、IP 键名、连接序号）
/// - `stop`: 停止信号
/// - `tx`: 广播通道发送端
///
/// # 说明
/// 根据 `spec.udp_mode` 执行发送或接收：
/// - 发送模式：从 QuadFrame 读取 4 槽数据，拼接为 380 字节发送
/// - 接收模式：接收 UDP 数据，通过帧提取器解析后发射事件
fn run_single_connection(
    qf: Arc<QuadFrame>,
    spec: &ConnSpec,
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) {
    let cfg = match config::global() {
        Some(c) => c,
        None => {
            error!("[{}] 配置未加载", spec.conn_idx);
            return;
        }
    };

    let local_ip = cfg.get_or("IP", &spec.local_key, "0.0.0.0");
    let remote_ip = cfg.get_or("IP", &spec.remote_key, "0.0.0.0");
    let port_key = parse_ip_to_port(&spec.remote_key)
        .unwrap_or_else(|| format!("Port{}", &spec.remote_key[2..]));
    let remote_port: u16 = cfg.get_or("IP", &port_key, "0").parse().unwrap_or(0);

    let udp = match UdpControl::create(&local_ip, &remote_ip, remote_port, spec.udp_mode) {
        Ok(u) => u,
        Err(e) => {
            error!("[UDP-{}] 创建失败: {}", spec.conn_idx, e);
            return;
        }
    };

    let result = Arc::new(Mutex::new(None::<ExtractedFrame>));
    let ext = result.clone();
    let mut fe = new_extractor(ext);
    let mut buf = [0u8; 4096];
    if spec.udp_mode == UdpMode::Recv {
        let _ = udp.set_recv_buffer_size(1024 * 1024);
    }
    let mut throttle = Throttle::new();
    let mut i: usize = 0;
    let local_key = spec.local_key.clone();
    let remote_key = spec.remote_key.clone();

    while !stop.load(Ordering::Relaxed) {
        match spec.udp_mode {
            UdpMode::Send => {
                let arrays: [[u8; FRAME_LEN]; 4] = [
                    qf.read_slot(0),
                    qf.read_slot(1),
                    qf.read_slot(2),
                    qf.read_slot(3),
                ];
                let data_vec = arrays.concat();
                let data: [u8; 380] = match data_vec.try_into() {
                    Ok(d) => d,
                    Err(_) => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                };
                if remote_ip == "226.0.0.1" {
                    // 特殊目标（与原项目一致）：发送设备帧（ComFTJ1CEquipment 91 字节）
                    let t = crate::ftj1c::com::ComFTJ1CEquipment;
                    if let Some(s) = parse_hex(t.build_frame(qf.clone()).as_str()) {
                        let _ = udp.send_to(&s);
                    }
                } else {
                    let _ = udp.send_to(&data);
                }
                if throttle.ready() {
                    let slot = i % 4;
                    emit(
                        &tx,
                        spec.conn_idx,
                        slot,
                        "-",
                        &local_key,
                        remote_port,
                        &remote_key,
                        remote_port,
                        format!("{}:[{}]{}", "接收", FRAME_LEN, format_hex(&qf.read_slot(slot))),
                        format!("{}:[{}]{}", "发送", data.len(), format_hex(&data)),
                    );
                    i = i.wrapping_add(1);
                }
                // 节流发送频率，避免 100% CPU 空转 + 全速灌包
                thread::sleep(Duration::from_millis(10));
            }
            UdpMode::Recv => match udp.recv_from(&mut buf) {
                Ok((_len, _)) => {
                    fe.feed(&buf[.._len]);
                    if let Some(ef) = result.lock().unwrap_or_else(|e| e.into_inner()).take() {
                        if throttle.ready() {
                            emit(
                                &tx,
                                spec.conn_idx,
                                ef.slot,
                                "-",
                                &local_key,
                                remote_port,
                                &remote_key,
                                remote_port,
                                format!("{}:[{}]{}", "接收", ef.data.len(), format_hex(&ef.data)),
                                format!("{}:[{}]{}", "发送", ef.data.len(), format_hex(&ef.data)),
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("[UDP-{}] recv error: {}", spec.conn_idx, e);
                    thread::sleep(Duration::from_millis(10));
                }
            },
        }
    }
    info!("[UDP-{}] 线程已停止", spec.conn_idx);
}

// ════════════════════════════════════════════════════════════
//  模拟模式（进程内数据源，无需硬件）
// ════════════════════════════════════════════════════════════

/// 生成模拟帧
///
/// # 参数
/// - `seq`: 帧序列号
/// - `conn_idx`: 连接序号（写入 payload[0]，用于区分来源）
/// - `slot_byte`: 槽位字节（1~4）
/// - `rng`: 随机数生成器
///
/// # 返回值
/// 95 字节模拟帧，包含帧头、槽位、序列号、随机载荷和校验和
fn build_mock_frame(seq: u32, conn_idx: usize, slot_byte: u8, rng: &mut impl Rng) -> [u8; FRAME_LEN] {
    let mut frame = [0u8; FRAME_LEN];
    frame[0..3].copy_from_slice(&HEADER); // 帧头 EB 90 5B
    frame[3] = slot_byte; // 槽位字节
    frame[4..8].copy_from_slice(&seq.to_le_bytes()); // 序列号
    frame[8] = conn_idx as u8; // 连接序号
    rng.fill(&mut frame[9..93]); // 随机载荷
    // 计算校验和
    let sum: u16 = frame[0..93].iter().map(|&b| b as u16).sum();
    frame[93..95].copy_from_slice(&sum.to_le_bytes());
    frame
}

/// 模拟主链线程（conn 5，来源 P）
///
/// # 说明
/// 每 200ms 生成 4 帧，写入 QuadFrame。
/// 在 5~10 秒窗口暂停，模拟主链故障，用于验证主备切换。
fn run_mock_primary(qf: Arc<QuadFrame>, stop: Arc<AtomicBool>, tx: broadcast::Sender<crate::common::ws::EventPayload>) {
    let mut rng = rand::thread_rng();
    let mut seq: u32 = 1;
    let start = Instant::now();
    let mut throttle = Throttle::new();

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(200));
        let elapsed = start.elapsed();
        // IP11（主链）在 5~10 秒窗口暂停，模拟主链故障
        if elapsed > Duration::from_secs(5) && elapsed < Duration::from_secs(10) {
            continue;
        }
        for slot_byte in 1..=4u8 {
            let frame = build_mock_frame(seq, 5, slot_byte, &mut rng);
            let accepted = qf.try_update((slot_byte - 1) as usize, SRC_PRIMARY, seq, &frame);
            if throttle.ready() {
                emit(
                    &tx,
                    5,
                    (slot_byte - 1) as usize,
                    "P",
                    "",
                    0,
                    "",
                    0,
                    format!(
                        "accepted?{}>{}:[{}]{}",
                        accepted,
                        "接收",
                        FRAME_LEN,
                        format_hex(&frame)
                    ),
                    format_hex(&frame),
                );
            }
        }
        seq = seq.wrapping_add(1);
    }
    info!("[Mock-Primary] 线程已停止");
}

/// 模拟备链线程（conn 7，来源 S）
///
/// # 说明
/// 每 200ms 生成 4 帧，持续发送。
/// 当主链心跳超时后，备链帧会被接受并发射到前端。
fn run_mock_secondary(qf: Arc<QuadFrame>, stop: Arc<AtomicBool>, tx: broadcast::Sender<crate::common::ws::EventPayload>) {
    let mut rng = rand::thread_rng();
    let mut seq: u32 = 1;
    let mut throttle = Throttle::new();

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(200));
        for slot_byte in 1..=4u8 {
            let frame = build_mock_frame(seq, 7, slot_byte, &mut rng);
            let accepted = qf.try_update((slot_byte - 1) as usize, SRC_SECONDARY, seq, &frame);
            if throttle.ready() {
                emit(
                    &tx,
                    7,
                    (slot_byte - 1) as usize,
                    "S",
                    "",
                    0,
                    "",
                    0,
                    format!(
                        "accepted?{}>{}:[{}]{}",
                        accepted,
                        "接收",
                        FRAME_LEN,
                        format_hex(&frame)
                    ),
                    format_hex(&frame),
                );
            }
        }
        seq = seq.wrapping_add(1);
    }
    info!("[Mock-Secondary] 线程已停止");
}

/// 模拟单路连接线程
///
/// # 说明
/// - 发送型（0/1/4/6）：从 QuadFrame 读取 4 槽数据，拼接为 380 字节
/// - 接收型（2/3）：直接生成模拟帧
fn run_mock_single(
    spec: &ConnSpec,
    qf: Arc<QuadFrame>,
    stop: Arc<AtomicBool>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) {
    let mut rng = rand::thread_rng();
    let mut seq: u32 = 1;
    let mut throttle = Throttle::new();
    let mut i: usize = 0;
    let local_key = spec.local_key.clone();
    let remote_key = spec.remote_key.clone();

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(200));
        match spec.udp_mode {
            UdpMode::Send => {
                // 与真实模式一致：发送 4 槽拼接的 380 字节，事件反映槽位数据
                let arrays: [[u8; FRAME_LEN]; 4] = [
                    qf.read_slot(0),
                    qf.read_slot(1),
                    qf.read_slot(2),
                    qf.read_slot(3),
                ];
                let data: [u8; 380] = arrays.concat().try_into().unwrap();
                if throttle.ready() {
                    let slot = i % 4;
                    emit(
                        &tx,
                        spec.conn_idx,
                        slot,
                        "-",
                        &local_key,
                        0,
                        &remote_key,
                        0,
                        format!("{}:[{}]{}", "接收", FRAME_LEN, format_hex(&qf.read_slot(slot))),
                        format!("{}:[{}]{}", "发送", data.len(), format_hex(&data)),
                    );
                    i = i.wrapping_add(1);
                }
            }
            UdpMode::Recv => {
                // 模拟接收到的帧（每 200ms 生成 4 槽中的一帧）
                for slot_byte in 1..=4u8 {
                    let frame = build_mock_frame(seq, spec.conn_idx, slot_byte, &mut rng);
                    if throttle.ready() {
                        emit(
                            &tx,
                            spec.conn_idx,
                            (slot_byte - 1) as usize,
                            "-",
                            &local_key,
                            0,
                            &remote_key,
                            0,
                            format!("{}:[{}]{}", "接收", frame.len(), format_hex(&frame)),
                            format!("{}:[{}]{}", "发送", frame.len(), format_hex(&frame)),
                        );
                    }
                }
            }
        }
        seq = seq.wrapping_add(1);
    }
    info!("[Mock-UDP-{}] 线程已停止", spec.conn_idx);
}

// ════════════════════════════════════════════════════════════
//  串口发送（3 路，从 demo-test3-ftj backend/process.rs 迁移）
// ════════════════════════════════════════════════════════════

/// 串口连接配置
///
/// # 说明
/// 定义串口发送线程的配置节名和连接序号。
/// 连接序号 8/9/10 在前端不展示，仅用于内部标识。
struct ComSpec {
    /// 配置节名（如 "ComFTJ1CTrajectory"）
    section: &'static str,
    /// 连接序号（8/9/10，前端不展示）
    conn_idx: usize,
}

/// 启动 3 路串口发送线程
///
/// # 说明
/// 创建 3 个串口发送线程，分别对应：
/// - 航迹帧（ComFTJ1CTrajectory）
/// - 设备帧（ComFTJ1CEquipment）
/// - 遥测帧（ComFTJ1CTelemetering）
fn run_coms(qf: Arc<QuadFrame>, stop: Arc<AtomicBool>) {
    let conns = vec![
        ComSpec { section: "ComFTJ1CTrajectory", conn_idx: 8 },
        ComSpec { section: "ComFTJ1CEquipment", conn_idx: 9 },
        ComSpec { section: "ComFTJ1CTelemetering", conn_idx: 10 },
    ];

    let count = conns.len();
    for spec in conns {
        let (qf, stop) = (qf.clone(), stop.clone());
        thread::spawn(move || run_one_com(qf, &spec, stop));
    }

    info!("[COM] 已启动 {} 路串口发送", count);
}

/// 单路串口发送线程
///
/// # 说明
/// 循环读取 QuadFrame 4 槽数据，拼接为 380 字节，通过串口发送。
/// 如果配置了协议构建器，还会构建对应的协议帧并输出。
fn run_one_com(qf: Arc<QuadFrame>, spec: &ComSpec, stop: Arc<AtomicBool>) {
    let com = match ComControl::create(spec.section) {
        Ok(c) => c,
        Err(e) => {
            error!("[COM-{}] {} 创建失败: {}", spec.conn_idx, spec.section, e);
            return;
        }
    };

    let transform: Option<Box<dyn ComFTJ1CBase>> = make_transform(spec.section);

    while !stop.load(Ordering::Relaxed) {
        let arrays: [[u8; FRAME_LEN]; 4] = [
            qf.read_slot(0),
            qf.read_slot(1),
            qf.read_slot(2),
            qf.read_slot(3),
        ];
        let data: [u8; 380] = match arrays.concat().try_into() {
            Ok(d) => d,
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
        };

        match com.send(&data) {
            Ok(_len) => {
                if let Some(ef) = transform.as_ref() {
                    debug!("{}==============={}", spec.section, ef.build_frame(qf.clone()));
                }
                // 10ms 循环下每秒约 100 条，日志级别降到 debug 避免刷屏
                debug!("{}发送:{}", spec.section, format_hex(&data));
                // 节流发送频率，避免串口发送线程 100% CPU 空转
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                error!("[COM-{}] 发送失败: {}", spec.conn_idx, e);
                thread::sleep(Duration::from_millis(10));
                continue;
            }
        }
    }

    info!("[COM-{}] 线程已停止", spec.conn_idx);
}
