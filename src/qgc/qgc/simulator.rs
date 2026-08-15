//! # 模拟飞控（多旋翼仿真器）
//!
//! `[Udp] Mock = true` 时启动：模拟一架 ArduPilot 多旋翼，通过**真实 MAVLink v2 帧**
//! 与地面站通信（UDP 回环），完整走一遍「编码 → 发送 → 接收 → 解码」链路，
//! 因此同一套代码对真实飞控（PX4 / ArduPilot）即插即用。
//!
//! ## 拓扑（Mock 模式）
//!
//! ```text
//! 模拟器套接字(127.0.0.1:LocalPort+1) ──遥测帧──▶ 地面站套接字(0.0.0.0:LocalPort)
//!        ◀──命令帧── 地面站发送目标（学习到的对端 = 模拟器地址）
//! ```
//!
//! ## 行为模型
//!
//! - 悬停 / 爬升（起飞命令） / 降落 / 返航（RTL）
//! - 任务航点飞行：依次飞向 MISSION_ITEM_INT 上传的航点（跳过 seq 0 首页）
//! - 支持命令：解锁/锁定、起飞、降落、返航、模式切换、任务上传/下载/清除
//! - 电池缓慢放电、姿态噪声、GPS 抖动（随机数用 xorshift32，无外部依赖）

use crate::qgc::mavlink::{self, FrameExtractor};
use crate::qgc::mission::MissionState;
use crate::qgc::models::QgcMissionItem;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// 简单 xorshift32 伪随机数生成器（线程内使用，避免外部依赖）
struct XorShift(u32);

impl XorShift {
    fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// [-1.0, 1.0) 均匀分布
    fn unit(&mut self) -> f32 {
        (self.next() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// 模拟飞行器状态
struct SimVehicle {
    sysid: u8,
    compid: u8,
    armed: bool,
    mode: u32,
    lat: f64,
    lon: f64,
    alt_msl: f64,
    alt_rel: f64,
    heading: f32,
    roll: f32,
    pitch: f32,
    vx: i16,
    vy: i16,
    vz: i16,
    groundspeed: f32,
    airspeed: f32,
    climb: f32,
    throttle: u16,
    voltage: f32,
    current: f32,
    remaining: i8,
    /// 已消耗电量（mAh，2A × 0.1s ≈ 0.0556 mAh/拍）
    consumed_mah: f32,
    /// 飞行时长（秒，解锁累计）
    flight_time_s: f32,
    takeoff_alt: f32,
    /// 当前目标（纬度, 经度, 相对高度）；None = 悬停
    target: Option<(f64, f64, f64)>,
    /// 任务暂停标志（DO_PAUSE_CONTINUE）
    paused: bool,
    /// 键盘/摇杆速度指令（机体 vx/vy/vz，m/s）；None = 无
    kbd_vel: Option<(f32, f32, f32)>,
    home: (f64, f64),
    mission: Vec<QgcMissionItem>,
    /// 正在执行的航点下标（1..=n，0 为首页不飞）
    wp_index: usize,
    /// 任务上传：期望接收的序号（0 起）
    upload_next: Option<u16>,
    upload_count: u16,
    seq: u8,
    time_boot_ms: u32,
    rng: XorShift,
}

impl SimVehicle {
    fn new() -> Self {
        // 默认起飞点（上海）
        let lat = 31.2304;
        let lon = 121.4737;
        Self {
            sysid: 1,
            compid: 1,
            armed: false,
            mode: 0,
            lat,
            lon,
            alt_msl: 20.0,
            alt_rel: 0.0,
            heading: 0.0,
            roll: 0.0,
            pitch: 0.0,
            vx: 0,
            vy: 0,
            vz: 0,
            groundspeed: 0.0,
            airspeed: 0.0,
            climb: 0.0,
            throttle: 0,
            voltage: 15.8,
            current: 0.0,
            remaining: 100,
            consumed_mah: 0.0,
            flight_time_s: 0.0,
            takeoff_alt: 0.0,
            target: None,
            paused: false,
            kbd_vel: None,
            home: (lat, lon),
            mission: Vec::new(),
            wp_index: 1,
            upload_next: None,
            upload_count: 0,
            seq: 0,
            time_boot_ms: 0,
            rng: XorShift::new(0x12345678),
        }
    }

    fn base_mode(&self) -> u8 {
        let mut m = mavlink::consts::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED;
        if self.armed {
            m |= mavlink::consts::MAV_MODE_FLAG_SAFETY_ARMED;
        }
        m
    }

    /// 每 100ms 推进一步飞行仿真
    fn tick(&mut self) {
        self.time_boot_ms += 100;
        // 电量放电（悬停电流 2A，满电电压 15.8V）
        if self.armed {
            self.remaining = (self.remaining as i32 - 2).clamp(0, 100) as i8;
            self.voltage = 15.8 - (100 - self.remaining) as f32 * 0.012;
            self.current = 2.0 + self.rng.unit().abs() * 0.5;
            // 2A × 0.1s ≈ 0.0556 mAh/拍
            self.consumed_mah += 0.0556;
            self.flight_time_s += 0.1;
        }
        // 任务目标选择
        let mut moving = false;
        let mut tx = 0.0;
        let mut ty = 0.0;
        let mut tz = 0.0;
        if self.armed && !self.paused {
            let mode = mavlink::mode_name(self.mode);
            match mode.as_str() {
                "AUTO" | "GUIDED" => {
                    // 起飞爬升优先（GUIDED + takeoff 命令）
                    if self.takeoff_alt > 0.0 && self.alt_rel < self.takeoff_alt as f64 {
                        tx = self.lat;
                        ty = self.lon;
                        tz = self.takeoff_alt as f64;
                        moving = true;
                    } else if self.wp_index < self.mission.len() {
                        let wp = &self.mission[self.wp_index];
                        tx = wp.lat;
                        ty = wp.lon;
                        tz = wp.altitude as f64;
                        moving = true;
                    }
                }
                "RTL" => {
                    tx = self.home.0;
                    ty = self.home.1;
                    tz = 30.0;
                    moving = true;
                }
                "LAND" => {
                    tx = self.lat;
                    ty = self.lon;
                    tz = 0.0;
                    moving = true;
                }
                _ => {}
            }
        }
        let last_lat = self.lat;
        let last_lon = self.lon;
        let last_alt = self.alt_rel;
        if moving {
            if let Some((lt, ln, az)) = self.target_position() {
                // 已有目标：向目标推进
                self.move_toward(lt, ln, az, 0.1);
                // 到位判定（水平 2m / 高度 0.5m）
                let d_h = haversine(self.lat, self.lon, lt, ln);
                let d_a = (self.alt_rel - az).abs();
                if d_h < 2.0 && d_a < 0.5 {
                    self.target = None;
                    self.wp_index = (self.wp_index + 1).min(self.mission.len().max(1) + 1);
                }
            } else {
                self.target = Some((tx, ty, tz));
            }
        } else {
            self.target = None;
        }
        // 键盘/摇杆速度指令（机体坐标系：前/右/下）
        if self.armed && !self.paused {
            if let Some((vx, vy, vz)) = self.kbd_vel {
                let dt = 0.1;
                let rad = self.heading.to_radians();
                // 机体前方（vx）→ 纬度方向（0° 朝北），机体右方（vy）→ 经度方向
                let dlat = (vx * rad.cos() - vy * rad.sin()) as f64 * dt / 111320.0;
                let dlon = (vx * rad.sin() + vy * rad.cos()) as f64 * dt / (111320.0 * self.lat.to_radians().cos().max(0.01));
                self.lat += dlat;
                self.lon += dlon;
                self.alt_rel = (self.alt_rel - (vz as f64) * dt).max(0.0);
            }
        }
        // 位置噪声（悬停抖动 ±0.000005°）
        self.lat += (self.rng.unit() as f64) * 5e-6;
        self.lon += (self.rng.unit() as f64) * 5e-6;
        // 速度 / 高度 / 姿态派生（dt = 0.1s：m → cm/s 需 ×1000）
        let dlat = (self.lat - last_lat) * 111320.0;
        let dlon = (self.lon - last_lon) * 111320.0 * (self.lat.to_radians().cos() as f64);
        let vx = (dlon * 1000.0) as i16;
        let vy = (dlat * 1000.0) as i16;
        let vz = ((self.alt_rel - last_alt) * 1000.0) as i16;
        self.vx = vx;
        self.vy = vy;
        self.vz = vz;
        self.groundspeed = ((vx as f32).powi(2) + (vy as f32).powi(2)).sqrt() / 100.0;
        self.airspeed = self.groundspeed * 0.98;
        self.climb = vz as f32 / 100.0;
        self.roll = 2.0 * (self.time_boot_ms as f32 / 500.0).sin() + self.rng.unit() * 0.5;
        self.pitch = 1.5 * (self.time_boot_ms as f32 / 700.0).sin() + self.rng.unit() * 0.5;
        self.throttle = if self.alt_rel < 0.5 { 0 } else { (45 + (self.climb * 20.0) as i32).clamp(0, 100) as u16 };
        self.alt_msl = 20.0 + self.alt_rel;
    }

    /// 目标推进（水平 5 m/s，垂直 2 m/s）
    fn move_toward(&mut self, lat: f64, lon: f64, alt: f64, dt: f64) {
        let bearing = bearing_deg(self.lat, self.lon, lat, lon);
        let speed = 5.0 * dt;
        let dlat = speed * bearing.to_radians().cos() / 111320.0;
        let dlon = speed * bearing.to_radians().sin() / (111320.0 * self.lat.to_radians().cos().max(0.01));
        self.lat += dlat;
        self.lon += dlon;
        let d_alt = (alt - self.alt_rel).clamp(-2.0 * dt, 2.0 * dt);
        self.alt_rel += d_alt;
        self.heading = bearing as f32;
        self.target = Some((lat, lon, alt));
    }

    fn target_position(&self) -> Option<(f64, f64, f64)> {
        self.target
    }

    /// 任务上传：GCS 发来 MISSION_ITEM_INT(seq)
    fn on_mission_item(&mut self, seq: u16, lat: f64, lon: f64, alt: f32) -> Option<Vec<u8>> {
        let Some(next) = self.upload_next else { return None };
        if seq != next {
            return None;
        }
        // 按 seq 存入（0 = 首页）
        while self.mission.len() <= seq as usize {
            self.mission.push(QgcMissionItem {
                seq: 0,
                command: mavlink::cmd::NAV_WAYPOINT,
                lat: 0.0,
                lon: 0.0,
                altitude: 0.0,
            });
        }
        self.mission[seq as usize] = QgcMissionItem {
            seq,
            command: mavlink::cmd::NAV_WAYPOINT,
            lat,
            lon,
            altitude: alt,
        };
        self.upload_next = Some(next + 1);
        let count = self.upload_count;
        if next + 1 >= count {
            // 全部接收完成 → ACK
            self.upload_next = None;
            self.wp_index = 1;
            debug!("[qgc] 模拟器任务上传完成（{} 条）", count);
            return Some(mavlink::encode_mission_ack(self.sysid, self.compid, self.next_seq(), 255, 190, mavlink::consts::MAV_MISSION_ACCEPTED));
        }
        // 继续请求下一条
        Some(mavlink::encode_mission_request_int(self.sysid, self.compid, self.next_seq(), 255, 190, next + 1))
    }

    fn next_seq(&mut self) -> u8 {
        let s = self.seq;
        self.seq = self.seq.wrapping_add(1);
        s
    }

    /// 处理一条来自地面站的命令，返回需要回发的帧
    fn handle_command(&mut self, msgid: u32, payload: &[u8]) -> Option<Vec<u8>> {
        match msgid {
            mavlink::msg::COMMAND_LONG => {
                let cmd = mavlink::decode_command_long(payload);
                let result = match cmd.command {
                    mavlink::cmd::COMPONENT_ARM_DISARM => {
                        self.armed = cmd.params[0] > 0.5;
                        info!("[qgc] 模拟器 {}", if self.armed { "解锁" } else { "锁定" });
                        0
                    }
                    mavlink::cmd::NAV_TAKEOFF => {
                        self.takeoff_alt = cmd.params[6].max(1.0);
                        self.target = None;
                        info!("[qgc] 模拟器起飞命令（高度 {}m）", self.takeoff_alt);
                        0
                    }
                    mavlink::cmd::NAV_LAND => {
                        self.takeoff_alt = 0.0;
                        self.mode = 9; // LAND
                        self.target = Some((self.lat, self.lon, 0.0));
                        info!("[qgc] 模拟器降落命令");
                        0
                    }
                    mavlink::cmd::NAV_RETURN_TO_LAUNCH => {
                        self.mode = 6; // RTL
                        self.takeoff_alt = 0.0;
                        self.paused = false;
                        self.kbd_vel = None;
                        self.target = Some((self.home.0, self.home.1, 30.0));
                        info!("[qgc] 模拟器返航命令");
                        0
                    }
                    mavlink::cmd::MISSION_START => {
                        self.mode = 3; // AUTO
                        self.paused = false;
                        self.kbd_vel = None;
                        self.wp_index = 1;
                        info!("[qgc] 模拟器开始执行任务");
                        0
                    }
                    mavlink::cmd::DO_PAUSE_CONTINUE => {
                        self.paused = cmd.params[0] < 0.5;
                        if self.paused {
                            self.target = None;
                            info!("[qgc] 模拟器任务暂停");
                        } else {
                            self.kbd_vel = None;
                            info!("[qgc] 模拟器任务继续");
                        }
                        0
                    }
                    _ => 1, // MAV_RESULT_UNSUPPORTED
                };
                Some(mavlink::encode_command_ack(self.sysid, self.compid, self.next_seq(), cmd.command, result))
            }
            mavlink::msg::SET_MODE => {
                let m = mavlink::decode_set_mode(payload);
                if m.base_mode & mavlink::consts::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED != 0 {
                    self.mode = m.custom_mode;
                    // 模式切换时重置目标（新模式下按 tick 逻辑重新选目标）
                    self.target = None;
                    if m.custom_mode == 6 || m.custom_mode == 9 {
                        self.takeoff_alt = 0.0;
                    }
                    info!("[qgc] 模拟器切换模式 → {}", mavlink::mode_name(m.custom_mode));
                }
                None
            }
            mavlink::msg::SET_POSITION_TARGET_GLOBAL_INT => {
                // 随点随行：GUIDED + 直接飞向目标坐标
                let g = mavlink::decode_set_position_global(payload);
                self.mode = 4; // GUIDED
                self.paused = false;
                self.kbd_vel = None;
                self.takeoff_alt = 0.0;
                self.target = Some((g.lat, g.lon, g.alt as f64));
                info!("[qgc] 模拟器随点随行 → ({:.5}, {:.5}, {}m)", g.lat, g.lon, g.alt);
                None
            }
            mavlink::msg::SET_POSITION_TARGET_LOCAL_NED => {
                // 键盘/摇杆速度控制（机体坐标系）
                let v = mavlink::decode_set_position_local(payload);
                self.kbd_vel = Some((v.vx, v.vy, v.vz));
                None
            }
            mavlink::msg::MISSION_COUNT => {
                let c = mavlink::decode_mission_count(payload);
                self.upload_count = c.count;
                self.upload_next = Some(0);
                // 请求第 0 条
                return Some(mavlink::encode_mission_request_int(self.sysid, self.compid, self.next_seq(), 255, 190, 0));
            }
            mavlink::msg::MISSION_ITEM_INT => {
                let it = mavlink::decode_mission_item_int(payload);
                return self.on_mission_item(it.seq, it.x as f64 / 1e7, it.y as f64 / 1e7, it.z);
            }
            mavlink::msg::MISSION_REQUEST_LIST => {
                let n = self.mission.len() as u16;
                return Some(mavlink::encode_mission_count(self.sysid, self.compid, self.next_seq(), 255, 190, n));
            }
            mavlink::msg::MISSION_REQUEST_INT => {
                let r = mavlink::decode_mission_request_int(payload);
                if let Some(item) = self.mission.get(r.seq as usize) {
                    let (lat, lon, alt) = (item.lat, item.lon, item.altitude);
                    return Some(mavlink::encode_mission_item_int(
                        self.sysid,
                        self.compid,
                        self.next_seq(),
                        255,
                        190,
                        r.seq,
                        lat,
                        lon,
                        alt,
                    ));
                }
                None
            }
            mavlink::msg::MISSION_CLEAR_ALL => {
                self.mission.clear();
                self.wp_index = 1;
                self.upload_next = None;
                info!("[qgc] 模拟器任务已清除");
                return Some(mavlink::encode_mission_ack(self.sysid, self.compid, self.next_seq(), 255, 190, mavlink::consts::MAV_MISSION_ACCEPTED));
            }
            _ => None,
        }
    }
}

/// 球面大圆距离（米）
fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().atan2((1.0 - a).sqrt())
}

/// 起点到终点的方位角（度）
fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * phi2.cos();
    let x = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * dl.cos();
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

/// 模拟器主循环（独立线程）
///
/// # 参数
/// - `local_port`：地面站监听端口（遥测发送目标）
/// - `stop`：停止信号
/// - `mission`：共享任务状态（写入 current_seq 供遥测展示）
pub fn run_simulator(local_port: u16, stop: Arc<AtomicBool>, mission: Arc<std::sync::Mutex<MissionState>>) {
    // 模拟器端口 = 本地端口 + 1（被占用时依次 +2..+5）
    let mut sim_port = local_port + 1;
    let socket = loop {
        match UdpSocket::bind(("127.0.0.1", sim_port)) {
            Ok(s) => break s,
            Err(e) if sim_port < local_port + 6 => {
                sim_port += 1;
                warn!("[qgc] 模拟器端口 {} 被占用: {}，尝试 {}", sim_port - 1, e, sim_port);
            }
            Err(e) => {
                warn!("[qgc] 模拟器端口绑定失败（连续尝试失败）: {}", e);
                return;
            }
        }
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(50)));
    let target: SocketAddr = format!("127.0.0.1:{}", local_port).parse().unwrap();
    info!("[qgc] 模拟飞控已启动（{} → 地面站 {}）", socket.local_addr().unwrap(), target);

    let mut vehicle = SimVehicle::new();
    let mut extractor = FrameExtractor::new();
    let mut last_hb = 0u32;
    let mut buf = vec![0u8; 2048];

    while !stop.load(Ordering::Relaxed) {
        // 接收并处理地面站命令
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, _src)) => {
                    for frame in extractor.feed(&buf[..n]) {
                        if let Some(reply) = vehicle.handle_command(frame.msgid, &frame.payload) {
                            let _ = socket.send_to(&reply, target);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => {
                    warn!("[qgc] 模拟器接收错误: {}", e);
                    break;
                }
            }
        }

        vehicle.tick();

        // 任务进度同步（当前执行航点）
        if let Ok(mut m) = mission.lock() {
            m.current_seq = if vehicle.armed && vehicle.wp_index < vehicle.mission.len() && matches!(mavlink::mode_name(vehicle.mode).as_str(), "AUTO" | "GUIDED") {
                vehicle.wp_index as i16
            } else {
                -1
            };
        }

        // 发送遥测帧（10Hz）
        let payload = telemetry_frames(&mut vehicle);
        for frame in &payload {
            let _ = socket.send_to(frame, target);
        }
        // 心跳（1Hz）
        if vehicle.time_boot_ms >= last_hb + 1000 {
            last_hb = vehicle.time_boot_ms;
            let hb = mavlink::encode_heartbeat(
                vehicle.sysid,
                vehicle.compid,
                vehicle.next_seq(),
                mavlink::consts::MAV_TYPE_QUADROTOR,
                mavlink::consts::MAV_AUTOPILOT_ARDUPILOTMEGA,
                vehicle.base_mode(),
                vehicle.mode,
                if vehicle.armed { mavlink::consts::MAV_STATE_ACTIVE } else { mavlink::consts::MAV_STATE_STANDBY },
            );
            let _ = socket.send_to(&hb, target);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    info!("[qgc] 模拟飞控已退出");
}

/// 生成一组遥测帧（ATTITUDE / GLOBAL_POSITION_INT / GPS_RAW_INT / VFR_HUD / SYS_STATUS / BATTERY_STATUS）
fn telemetry_frames(v: &mut SimVehicle) -> Vec<Vec<u8>> {
    let mut frames = Vec::with_capacity(6);
    // ATTITUDE（含角速率：滚转/俯仰小幅摆动，偏航恒 0）
    {
        let mut p = vec![0u8; 28];
        p[0..4].copy_from_slice(&v.time_boot_ms.to_le_bytes());
        p[4..8].copy_from_slice(&v.roll.to_radians().to_bits().to_le_bytes());
        p[8..12].copy_from_slice(&v.pitch.to_radians().to_bits().to_le_bytes());
        p[12..16].copy_from_slice(&v.heading.to_radians().to_bits().to_le_bytes());
        p[16..20].copy_from_slice(&(0.15 * (v.time_boot_ms as f32 / 500.0).cos()).to_bits().to_le_bytes());
        p[20..24].copy_from_slice(&(0.1 * (v.time_boot_ms as f32 / 700.0).cos()).to_bits().to_le_bytes());
        p[24..28].copy_from_slice(&0f32.to_bits().to_le_bytes());
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::ATTITUDE, &p));
    }
    // GLOBAL_POSITION_INT
    {
        let mut p = vec![0u8; 28];
        p[0..4].copy_from_slice(&v.time_boot_ms.to_le_bytes());
        p[4..8].copy_from_slice(&((v.lat * 1e7) as i32).to_le_bytes());
        p[8..12].copy_from_slice(&((v.lon * 1e7) as i32).to_le_bytes());
        p[12..16].copy_from_slice(&((v.alt_msl * 1000.0) as i32).to_le_bytes());
        p[16..20].copy_from_slice(&((v.alt_rel * 1000.0) as i32).to_le_bytes());
        p[20..22].copy_from_slice(&v.vx.to_le_bytes());
        p[22..24].copy_from_slice(&v.vy.to_le_bytes());
        p[24..26].copy_from_slice(&v.vz.to_le_bytes());
        p[26..28].copy_from_slice(&((v.heading * 100.0) as u16).to_le_bytes());
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::GLOBAL_POSITION_INT, &p));
    }
    // GPS_RAW_INT
    {
        let mut p = vec![0u8; 30];
        p[8] = 3; // fix_type = 3D
        p[9..13].copy_from_slice(&((v.lat * 1e7) as i32).to_le_bytes());
        p[13..17].copy_from_slice(&((v.lon * 1e7) as i32).to_le_bytes());
        p[17..21].copy_from_slice(&((v.alt_msl * 1000.0) as i32).to_le_bytes());
        p[21..23].copy_from_slice(&100u16.to_le_bytes()); // eph = 1.0
        p[23..25].copy_from_slice(&120u16.to_le_bytes()); // epv
        p[25..27].copy_from_slice(&((v.groundspeed * 100.0) as u16).to_le_bytes());
        p[27..29].copy_from_slice(&((v.heading * 100.0) as u16).to_le_bytes());
        p[29] = 12; // satellites_visible
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::GPS_RAW_INT, &p));
    }
    // VFR_HUD
    {
        let mut p = vec![0u8; 20];
        p[0..4].copy_from_slice(&v.airspeed.to_bits().to_le_bytes());
        p[4..8].copy_from_slice(&v.groundspeed.to_bits().to_le_bytes());
        p[8..10].copy_from_slice(&(v.heading as i16).to_le_bytes());
        p[10..12].copy_from_slice(&v.throttle.to_le_bytes());
        p[12..16].copy_from_slice(&(v.alt_msl as f32).to_bits().to_le_bytes());
        p[16..20].copy_from_slice(&v.climb.to_bits().to_le_bytes());
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::VFR_HUD, &p));
    }
    // SYS_STATUS（负载 32%~48% 波动）
    {
        let mut p = vec![0u8; 31];
        for o in [0usize, 4, 8] {
            p[o..o + 4].copy_from_slice(&0xFFFFu32.to_le_bytes());
        }
        let load = (320.0 + v.rng.unit().abs() * 160.0) as u16;
        p[12..14].copy_from_slice(&load.to_le_bytes());
        p[14..16].copy_from_slice(&((v.voltage * 1000.0) as u16).to_le_bytes());
        p[16..18].copy_from_slice(&((v.current * 100.0) as i16).to_le_bytes());
        p[18] = v.remaining as u8;
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::SYS_STATUS, &p));
    }
    // BATTERY_STATUS（v2 布局：remaining@35，current_consumed@27..31，time_remaining@36..40，charge_state@40）
    {
        let mut p = vec![0u8; 41];
        p[0] = 0; // id
        p[1] = 1; // MAV_BATTERY_FUNCTION_ALL
        p[2] = 1; // MAV_BATTERY_TYPE_LIPO
        p[3..5].copy_from_slice(&2000i16.to_le_bytes()); // 20°C
        p[5..7].copy_from_slice(&((v.voltage * 1000.0) as u16).to_le_bytes());
        p[25..27].copy_from_slice(&((v.current * 100.0) as i16).to_le_bytes());
        p[27..31].copy_from_slice(&((v.consumed_mah * 10.0) as i32).to_le_bytes()); // current_consumed (mAh)
        p[35] = v.remaining as u8;
        p[36..40].copy_from_slice(&(-1i32).to_le_bytes()); // time_remaining 未知
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::BATTERY_STATUS, &p));
    }
    // RADIO_STATUS（数传信号：本地/远端 rssi 波动）
    {
        let mut p = vec![0u8; 9];
        p[2] = (95 - (v.rng.unit().abs() * 10.0) as u8) as u8; // rssi
        p[3] = (88 - (v.rng.unit().abs() * 12.0) as u8) as u8; // remrssi
        p[4] = (70 + (v.rng.unit().abs() * 20.0) as u8) as u8; // txbuf
        p[5] = 100; // noise
        p[6] = 100; // remnoise
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::RADIO_STATUS, &p));
    }
    // HOME_POSITION（起飞点）
    {
        let mut p = vec![0u8; 66];
        p[4..8].copy_from_slice(&((v.home.0 * 1e7) as i32).to_le_bytes());
        p[8..12].copy_from_slice(&((v.home.1 * 1e7) as i32).to_le_bytes());
        p[12..16].copy_from_slice(&((v.alt_msl * 1000.0) as i32).to_le_bytes());
        frames.push(mavlink::encode_v2(v.sysid, v.compid, v.next_seq(), mavlink::msg::HOME_POSITION, &p));
    }
    frames
}
