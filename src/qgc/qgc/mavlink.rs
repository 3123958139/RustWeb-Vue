//! # MAVLink v2 帧编解码（消息子集）
//!
//! 实现 MAVLink v2 协议的帧格式解析与常用消息的编解码，
//! 兼容 MAVLink v1 帧（v1 载荷较短，按短载荷解码，缺失字段按 0 处理）。
//! 模拟器生成的帧也走本模块，保证真实飞控（PX4 / ArduPilot）即插即用。
//!
//! ## 帧格式（v2）
//!
//! ```text
//! Offset  Len  Field
//! ─────────────────────────────
//! 0       1    STX = 0xFD
//! 1       1    PAYLOAD_LEN（尾部全 0 字节会被裁剪）
//! 2       1    INCOMPAT_FLAGS（bit0 = 签名）
//! 3       1    COMPAT_FLAGS
//! 4       1    SEQ
//! 5       1    SYSID
//! 6       1    COMPID
//! 7-9     3    MSGID（小端）
//! 10..    n    PAYLOAD
//! ...     2    CHECKSUM（X25，覆盖：帧头去 STX + 载荷 + CRC_EXTRA）
//! ...     13   签名块（仅 INCOMPAT_FLAGS 置位时，本实现跳过不校验）
//! ```
//!
//! ## CRC_EXTRA 表
//!
//! 由官方 `mavlink/message_definitions` XML 经 mavgen 生成（c_library_v2
//! `MAVLINK_MESSAGE_CRCS`），本模块只登记用到的消息；CRC 算法：
//! X25 初值 0xFFFF，覆盖「帧头去掉 STX 字节 + 载荷（裁剪后）+ CRC_EXTRA」。
//! 数据来源：mavlink/c_library_v2 common.h（2026-08 核对）。

/// 消息 ID 常量（只登记本地面站用到的消息）
pub mod msg {
    pub const HEARTBEAT: u32 = 0;
    pub const SYS_STATUS: u32 = 1;
    pub const GPS_RAW_INT: u32 = 24;
    pub const ATTITUDE: u32 = 30;
    pub const GLOBAL_POSITION_INT: u32 = 33;
    pub const MISSION_REQUEST_LIST: u32 = 43;
    pub const MISSION_COUNT: u32 = 44;
    pub const MISSION_CLEAR_ALL: u32 = 45;
    pub const MISSION_ACK: u32 = 47;
    pub const MISSION_REQUEST_INT: u32 = 51;
    pub const REQUEST_DATA_STREAM: u32 = 66;
    pub const MISSION_ITEM_INT: u32 = 73;
    pub const VFR_HUD: u32 = 74;
    pub const COMMAND_LONG: u32 = 76;
    pub const COMMAND_ACK: u32 = 77;
    pub const SET_POSITION_TARGET_LOCAL_NED: u32 = 84;
    pub const SET_POSITION_TARGET_GLOBAL_INT: u32 = 86;
    pub const RADIO_STATUS: u32 = 109;
    pub const BATTERY_STATUS: u32 = 147;
    pub const SET_MODE: u32 = 176;
    pub const HOME_POSITION: u32 = 242;
    /// 参数协议：请求读取全部参数（飞控逐个回 PARAM_VALUE）
    pub const PARAM_REQUEST_LIST: u32 = 21;
    /// 参数协议：飞控回执单个参数
    pub const PARAM_VALUE: u32 = 22;
    /// 参数协议：地面站写入参数
    pub const PARAM_SET: u32 = 23;
    /// EKF 状态报告（传感器健康度）
    pub const EKF_STATUS_REPORT: u32 = 193;
    /// 振动数据（传感器健康度）
    pub const VIBRATION: u32 = 241;
    /// RC 通道（遥控器输入，含 rssi）
    pub const RC_CHANNELS: u32 = 65;
}

/// MAVLink 命令常量（MAV_CMD，COMMAND_LONG 的 command 字段）
pub mod cmd {
    pub const NAV_WAYPOINT: u16 = 16;
    pub const NAV_RETURN_TO_LAUNCH: u16 = 20;
    pub const NAV_LAND: u16 = 21;
    pub const NAV_TAKEOFF: u16 = 22;
    pub const DO_PAUSE_CONTINUE: u16 = 157;
    pub const MISSION_START: u16 = 300;
    pub const COMPONENT_ARM_DISARM: u16 = 400;
    pub const PREFLIGHT_CALIBRATION: u16 = 241;
    pub const DO_SET_SERVO: u16 = 183;
    pub const DO_SET_CAM_TRIGG_INTERVAL: u16 = 20001;
}

/// SET_POSITION_TARGET 掩码（type_mask，POSITION_TARGET_TYPEMASK 位定义）
///
/// 位布局：bit0-2 位置 xyz / bit3-5 速度 xyz / bit6-8 加速度 xyz /
/// bit10 偏航 / bit11 偏航速率。设位 = 忽略该分量。
pub mod mask {
    /// 忽略位置分量（只设置速度，键盘摇杆用）
    pub const POSITION_IGNORE: u16 = 0x07;
    /// 忽略速度分量（只设置位置，随点随行用）
    pub const VELOCITY_IGNORE: u16 = 0x38;
    /// 忽略加速度分量
    pub const ACCEL_IGNORE: u16 = 0x1C0;
    /// 忽略偏航
    pub const YAW_IGNORE: u16 = 0x400;
    /// 忽略偏航速率
    pub const YAW_RATE_IGNORE: u16 = 0x800;
}

/// 其他协议常量
pub mod consts {
    /// MAV_MODE_FLAG_SAFETY_ARMED：解锁位（心跳 base_mode）
    pub const MAV_MODE_FLAG_SAFETY_ARMED: u8 = 0x80;
    /// MAV_MODE_FLAG_CUSTOM_MODE_ENABLED：SET_MODE 的基础模式位（ArduPilot 要求）
    pub const MAV_MODE_FLAG_CUSTOM_MODE_ENABLED: u8 = 0x01;
    /// MAV_TYPE_GCS：地面站类型（心跳发送）
    pub const MAV_TYPE_GCS: u8 = 6;
    /// MAV_TYPE_QUADROTOR：四旋翼
    pub const MAV_TYPE_QUADROTOR: u8 = 2;
    /// MAV_AUTOPILOT_INVALID：非飞控组件（地面站心跳）
    pub const MAV_AUTOPILOT_INVALID: u8 = 8;
    /// MAV_AUTOPILOT_ARDUPILOTMEGA：ArduPilot
    pub const MAV_AUTOPILOT_ARDUPILOTMEGA: u8 = 3;
    /// MAV_STATE_STANDBY / MAV_STATE_ACTIVE：系统状态
    pub const MAV_STATE_STANDBY: u8 = 3;
    pub const MAV_STATE_ACTIVE: u8 = 4;
    /// MAV_DATA_STREAM_ALL：请求全数据流
    pub const MAV_DATA_STREAM_ALL: u8 = 0;
    /// MISSION_TYPE_MISSION：常规任务
    pub const MISSION_TYPE_MISSION: u8 = 0;
    /// MAV_MISSION_ACCEPTED：任务 ack 成功
    pub const MAV_MISSION_ACCEPTED: u8 = 0;
    /// MAV_PARAM_TYPE_REAL32：参数类型（单精度浮点，ArduPilot 参数通用类型）
    pub const MAV_PARAM_TYPE_REAL32: u8 = 9;
}

/// 各消息的 CRC_EXTRA（来源：c_library_v2 common.h `MAVLINK_MESSAGE_CRCS`）
pub fn crc_extra(msgid: u32) -> u8 {
    match msgid {
        msg::HEARTBEAT => 50,
        msg::SYS_STATUS => 124,
        msg::GPS_RAW_INT => 24,
        msg::ATTITUDE => 39,
        msg::GLOBAL_POSITION_INT => 104,
        msg::MISSION_REQUEST_LIST => 132,
        msg::MISSION_COUNT => 221,
        msg::MISSION_CLEAR_ALL => 232,
        msg::MISSION_ACK => 153,
        msg::MISSION_REQUEST_INT => 196,
        msg::REQUEST_DATA_STREAM => 148,
        msg::MISSION_ITEM_INT => 38,
        msg::VFR_HUD => 20,
        msg::COMMAND_LONG => 152,
        msg::COMMAND_ACK => 143,
        msg::SET_POSITION_TARGET_LOCAL_NED => 53,
        msg::SET_POSITION_TARGET_GLOBAL_INT => 53,
        msg::RADIO_STATUS => 185,
        msg::BATTERY_STATUS => 154,
        msg::SET_MODE => 89,
        msg::HOME_POSITION => 104,
        // 未登记消息按 0 计算（不校验其 CRC，直接跳过解码）
        _ => 0,
    }
}

/// 一帧解析结果
#[derive(Debug, Clone)]
pub struct MavFrame {
    pub msgid: u32,
    pub sysid: u8,
    pub compid: u8,
    /// 载荷（裁剪后长度与帧头 len 一致；v1 短载荷按短解析）
    pub payload: Vec<u8>,
}

/// X25 CRC 累加器（MAVLink 实现，与 crc_accumulate 等价）
struct X25 {
    crc: u16,
}

impl X25 {
    fn new() -> Self {
        Self { crc: 0xffff }
    }

    fn accumulate(&mut self, data: &[u8]) {
        for &b in data {
            let mut tmp = (b as u16) ^ (self.crc & 0xff);
            tmp ^= tmp << 4;
            self.crc = (self.crc >> 8) ^ (tmp << 8) ^ (tmp << 3) ^ (tmp >> 4);
        }
    }
}

/// 校验一帧 CRC（v2：帧头去 STX + 载荷 + CRC_EXTRA）
fn frame_crc_valid(header: &[u8], payload: &[u8], crc_bytes: &[u8], msgid: u32) -> bool {
    let mut x = X25::new();
    x.accumulate(header);
    x.accumulate(payload);
    x.accumulate(&[crc_extra(msgid)]);
    x.crc.to_le_bytes() == [crc_bytes[0], crc_bytes[1]]
}

/// 帧提取器：字节流 → MAVLink 帧
///
/// 内部维护累积缓冲，兼容 v1（0xFE）/ v2（0xFD）帧；
/// 含签名的 v2 帧（INCOMPAT_FLAGS bit0）跳过 13 字节签名块（不校验签名）。
/// 未登记消息不做 CRC 校验（无法查表），直接按长度消费。
#[derive(Default)]
pub struct FrameExtractor {
    buf: Vec<u8>,
}

impl FrameExtractor {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(1024) }
    }

    /// 喂入一段字节，返回解析出的完整帧（可能为 0..n 个）
    pub fn feed(&mut self, data: &[u8]) -> Vec<MavFrame> {
        self.buf.extend_from_slice(data);
        let mut frames = Vec::new();
        loop {
            match self.try_extract_one() {
                Some(frame) => frames.push(frame),
                None => break,
            }
        }
        frames
    }

    /// 尝试从缓冲开头提取一帧（失败返回 None 等待更多数据）
    fn try_extract_one(&mut self) -> Option<MavFrame> {
        // 1. 重同步：丢弃开头非魔数字节
        while !self.buf.is_empty() && self.buf[0] != 0xFD && self.buf[0] != 0xFE {
            self.buf.remove(0);
        }
        if self.buf.len() < 3 {
            return None;
        }
        let v2 = self.buf[0] == 0xFD;
        // 帧头长度：v1 = 6（STX..MSGID1），v2 = 10（STX..MSGID3）
        let header_len = if v2 { 10 } else { 6 };
        if self.buf.len() < header_len + 1 {
            return None;
        }
        let payload_len = self.buf[1] as usize;
        let signature_len = if v2 && (self.buf[2] & 0x01) != 0 { 13 } else { 0 };
        let total = header_len + payload_len + 2 + signature_len;
        if self.buf.len() < total {
            return None;
        }
        let (msgid, sysid, compid) = if v2 {
            (
                (self.buf[7] as u32) | ((self.buf[8] as u32) << 8) | ((self.buf[9] as u32) << 16),
                self.buf[5],
                self.buf[6],
            )
        } else {
            (self.buf[5] as u32, self.buf[3], self.buf[4])
        };
        let payload = self.buf[header_len..header_len + payload_len].to_vec();
        let crc_idx = header_len + payload_len;
        let crc = [self.buf[crc_idx], self.buf[crc_idx + 1]];
        // 校验（已知消息严格校验，未知消息直接放行）
        let known = crc_extra(msgid) != 0;
        if known && !frame_crc_valid(&self.buf[1..header_len], &payload, &crc, msgid) {
            // CRC 失败：丢弃该帧的第一个字节重新同步（可能是错位帧）
            self.buf.remove(0);
            return None;
        }
        let frame = MavFrame { msgid, sysid, compid, payload };
        self.buf.drain(..total);
        Some(frame)
    }
}

/// 编码 MAVLink v2 帧（载荷尾部全 0 裁剪，CRC 覆盖去 STX 的帧头 + 载荷 + CRC_EXTRA）
pub fn encode_v2(sysid: u8, compid: u8, seq: u8, msgid: u32, payload: &[u8]) -> Vec<u8> {
    let mut len = payload.len();
    while len > 1 && payload[len - 1] == 0 {
        len -= 1;
    }
    let mut frame = Vec::with_capacity(10 + len + 2);
    frame.push(0xFD);
    frame.push(len as u8);
    frame.push(0x00); // incompat_flags（无签名）
    frame.push(0x00); // compat_flags
    frame.push(seq);
    frame.push(sysid);
    frame.push(compid);
    frame.push((msgid & 0xFF) as u8);
    frame.push(((msgid >> 8) & 0xFF) as u8);
    frame.push(((msgid >> 16) & 0xFF) as u8);
    frame.extend_from_slice(&payload[..len]);
    let mut x = X25::new();
    x.accumulate(&frame[1..]);
    x.accumulate(&[crc_extra(msgid)]);
    frame.push((x.crc & 0xFF) as u8);
    frame.push((x.crc >> 8) as u8);
    frame
}

// ============ 小端读取工具 ============

fn rd_u8(p: &[u8], o: usize) -> u8 {
    p.get(o).copied().unwrap_or(0)
}

fn rd_u16(p: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([rd_u8(p, o), rd_u8(p, o + 1)])
}

fn rd_i16(p: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([rd_u8(p, o), rd_u8(p, o + 1)])
}

fn rd_u32(p: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([rd_u8(p, o), rd_u8(p, o + 1), rd_u8(p, o + 2), rd_u8(p, o + 3)])
}

fn rd_i32(p: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([rd_u8(p, o), rd_u8(p, o + 1), rd_u8(p, o + 2), rd_u8(p, o + 3)])
}

fn rd_f32(p: &[u8], o: usize) -> f32 {
    f32::from_bits(rd_u32(p, o))
}

// ============ 上行消息解码（飞控 → 地面站） ============

/// HEARTBEAT 解码结果
#[derive(Debug, Clone)]
pub struct Heartbeat {
    pub vehicle_type: u8,
    pub base_mode: u8,
    pub custom_mode: u32,
}

pub fn decode_heartbeat(p: &[u8]) -> Heartbeat {
    Heartbeat {
        vehicle_type: rd_u8(p, 0),
        base_mode: rd_u8(p, 2),
        custom_mode: rd_u32(p, 3),
    }
}

/// ATTITUDE 解码结果（弧度，角速率为弧度/秒）
#[derive(Debug, Clone)]
pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub rollspeed: f32,
    pub pitchspeed: f32,
    pub yawspeed: f32,
}

pub fn decode_attitude(p: &[u8]) -> Attitude {
    Attitude {
        roll: rd_f32(p, 4),
        pitch: rd_f32(p, 8),
        yaw: rd_f32(p, 12),
        rollspeed: rd_f32(p, 16),
        pitchspeed: rd_f32(p, 20),
        yawspeed: rd_f32(p, 24),
    }
}

/// GLOBAL_POSITION_INT 解码结果
#[derive(Debug, Clone)]
pub struct GlobalPositionInt {
    pub lat: i32,
    pub lon: i32,
    pub alt: i32,
    pub relative_alt: i32,
    pub vx: i16,
    pub vy: i16,
}

pub fn decode_global_position_int(p: &[u8]) -> GlobalPositionInt {
    GlobalPositionInt {
        lat: rd_i32(p, 4),
        lon: rd_i32(p, 8),
        alt: rd_i32(p, 12),
        relative_alt: rd_i32(p, 16),
        vx: rd_i16(p, 20),
        vy: rd_i16(p, 22),
    }
}

/// GPS_RAW_INT 解码结果
#[derive(Debug, Clone)]
pub struct GpsRawInt {
    pub fix_type: u8,
    pub lat: i32,
    pub lon: i32,
    pub alt: i32,
    /// 水平定位精度（mm）
    pub eph: u16,
    pub satellites_visible: u8,
}

pub fn decode_gps_raw_int(p: &[u8]) -> GpsRawInt {
    GpsRawInt {
        fix_type: rd_u8(p, 8),
        lat: rd_i32(p, 9),
        lon: rd_i32(p, 13),
        alt: rd_i32(p, 17),
        eph: rd_u16(p, 21),
        satellites_visible: rd_u8(p, 29),
    }
}

/// VFR_HUD 解码结果
#[derive(Debug, Clone)]
pub struct VfrHud {
    pub airspeed: f32,
    pub groundspeed: f32,
    pub heading: i16,
    pub alt: f32,
    pub climb: f32,
    /// 油门百分比（0~100）
    pub throttle: u16,
}

pub fn decode_vfr_hud(p: &[u8]) -> VfrHud {
    VfrHud {
        airspeed: rd_f32(p, 0),
        groundspeed: rd_f32(p, 4),
        heading: rd_i16(p, 8),
        alt: rd_f32(p, 12),
        climb: rd_f32(p, 16),
        throttle: rd_u16(p, 10),
    }
}

/// RADIO_STATUS 解码结果
#[derive(Debug, Clone)]
pub struct RadioStatus {
    /// 本地接收信号强度（dBm，127 表示未知）
    pub rssi: i8,
    /// 远端（飞控）接收信号强度（dBm，127 表示未知）
    pub remote_rssi: i8,
}

pub fn decode_radio_status(p: &[u8]) -> RadioStatus {
    RadioStatus {
        rssi: rd_u8(p, 2) as i8,
        remote_rssi: rd_u8(p, 3) as i8,
    }
}

/// HOME_POSITION 解码结果
#[derive(Debug, Clone)]
pub struct HomePosition {
    pub lat: i32,
    pub lon: i32,
    /// 海拔（mm，MSL）
    pub alt: i32,
}

pub fn decode_home_position(p: &[u8]) -> HomePosition {
    HomePosition {
        lat: rd_i32(p, 4),
        lon: rd_i32(p, 8),
        alt: rd_i32(p, 12),
    }
}

/// SET_POSITION_TARGET_GLOBAL_INT 解码结果（随点随行目标）
#[derive(Debug, Clone)]
pub struct SetPositionGlobal {
    pub lat: f64,
    pub lon: f64,
    pub alt: f32,
}

pub fn decode_set_position_global(p: &[u8]) -> SetPositionGlobal {
    SetPositionGlobal {
        lat: rd_i32(p, 9) as f64 / 1e7,
        lon: rd_i32(p, 13) as f64 / 1e7,
        alt: rd_f32(p, 17),
    }
}

/// SET_POSITION_TARGET_LOCAL_NED 解码结果（键盘/摇杆速度控制，机体坐标系）
#[derive(Debug, Clone)]
pub struct SetPositionLocal {
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}

pub fn decode_set_position_local(p: &[u8]) -> SetPositionLocal {
    SetPositionLocal {
        vx: rd_f32(p, 21),
        vy: rd_f32(p, 25),
        vz: rd_f32(p, 29),
    }
}

/// SYS_STATUS 解码结果
#[derive(Debug, Clone)]
pub struct SysStatus {    /// 飞控负载（×0.1%）
    pub load: u16,
    pub voltage_battery: u16,
    pub current_battery: i16,
    pub battery_remaining: i8,
}

pub fn decode_sys_status(p: &[u8]) -> SysStatus {
    SysStatus {
        load: rd_u16(p, 12),
        voltage_battery: rd_u16(p, 14),
        current_battery: rd_i16(p, 16),
        battery_remaining: rd_i8(p, 18),
    }
}

/// BATTERY_STATUS 解码结果（voltages[0] 主电压，v1 布局：remaining@35）
#[derive(Debug, Clone)]
pub struct BatteryStatus {
    pub voltage: u16,
    pub current_battery: i16,
    /// 已消耗电量（mAh）
    pub current_consumed: i32,
    pub battery_remaining: i8,
}

pub fn decode_battery_status(p: &[u8]) -> BatteryStatus {
    BatteryStatus {
        voltage: rd_u16(p, 5),
        current_battery: rd_i16(p, 25),
        current_consumed: rd_i32(p, 27),
        battery_remaining: rd_i8(p, 35),
    }
}

/// COMMAND_ACK 解码结果
#[derive(Debug, Clone)]
pub struct CommandAck {
    pub command: u16,
    pub result: u8,
}

pub fn decode_command_ack(p: &[u8]) -> CommandAck {
    CommandAck {
        command: rd_u16(p, 0),
        result: rd_u8(p, 2),
    }
}

/// MISSION_COUNT 解码结果
#[derive(Debug, Clone)]
pub struct MissionCount {
    pub count: u16,
    pub mission_type: u8,
}

pub fn decode_mission_count(p: &[u8]) -> MissionCount {
    MissionCount {
        count: rd_u16(p, 2),
        mission_type: rd_u8(p, 4),
    }
}

/// MISSION_REQUEST_INT 解码结果
#[derive(Debug, Clone)]
pub struct MissionRequestInt {
    pub seq: u16,
    pub mission_type: u8,
}

pub fn decode_mission_request_int(p: &[u8]) -> MissionRequestInt {
    MissionRequestInt {
        seq: rd_u16(p, 2),
        mission_type: rd_u8(p, 4),
    }
}

/// MISSION_ITEM_INT 解码结果
#[derive(Debug, Clone)]
pub struct MissionItemInt {
    pub seq: u16,
    pub command: u16,
    /// param1：悬停时间（秒，NAV_WAYPOINT）/ DO 命令参数
    pub param1: f32,
    /// param2：转弯半径（NAV_WAYPOINT）/ DO 命令参数
    pub param2: f32,
    /// param4：目标偏航角（度，NAV_WAYPOINT）/ DO 命令参数
    pub param4: f32,
    pub x: i32,
    pub y: i32,
    pub z: f32,
    pub mission_type: u8,
}

pub fn decode_mission_item_int(p: &[u8]) -> MissionItemInt {
    MissionItemInt {
        seq: rd_u16(p, 2),
        command: rd_u16(p, 5),
        param1: rd_f32(p, 9),
        param2: rd_f32(p, 13),
        param4: rd_f32(p, 21),
        x: rd_i32(p, 25),
        y: rd_i32(p, 29),
        z: rd_f32(p, 33),
        mission_type: rd_u8(p, 37),
    }
}

/// MISSION_ACK 解码结果
#[derive(Debug, Clone)]
pub struct MissionAck {
    pub result: u8,
    pub mission_type: u8,
}

pub fn decode_mission_ack(p: &[u8]) -> MissionAck {
    MissionAck {
        result: rd_u8(p, 2),
        mission_type: rd_u8(p, 3),
    }
}

/// COMMAND_LONG 解码结果（模拟器接收命令用）
#[derive(Debug, Clone)]
pub struct CommandLong {
    pub command: u16,
    pub params: [f32; 7],
}

pub fn decode_command_long(p: &[u8]) -> CommandLong {
    CommandLong {
        command: rd_u16(p, 2),
        params: [
            rd_f32(p, 5),
            rd_f32(p, 9),
            rd_f32(p, 13),
            rd_f32(p, 17),
            rd_f32(p, 21),
            rd_f32(p, 25),
            rd_f32(p, 29),
        ],
    }
}

/// SET_MODE 解码结果（模拟器接收模式切换用）
#[derive(Debug, Clone)]
pub struct SetMode {
    pub base_mode: u8,
    pub custom_mode: u32,
}

pub fn decode_set_mode(p: &[u8]) -> SetMode {
    SetMode {
        base_mode: rd_u8(p, 1),
        custom_mode: rd_u32(p, 2),
    }
}

fn rd_i8(p: &[u8], o: usize) -> i8 {
    rd_u8(p, o) as i8
}

// ============ 下行消息编码（地面站 → 飞控） ============

/// 编码 HEARTBEAT（地面站心跳）
pub fn encode_heartbeat(
    sysid: u8,
    compid: u8,
    seq: u8,
    vehicle_type: u8,
    autopilot: u8,
    base_mode: u8,
    custom_mode: u32,
    system_status: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 9];
    p[0] = vehicle_type;
    p[1] = autopilot;
    p[2] = base_mode;
    p[3..7].copy_from_slice(&custom_mode.to_le_bytes());
    p[7] = system_status;
    p[8] = 3; // mavlink_version
    encode_v2(sysid, compid, seq, msg::HEARTBEAT, &p)
}

/// 编码 COMMAND_LONG（7 参数命令）
pub fn encode_command_long(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    command: u16,
    params: [f32; 7],
) -> Vec<u8> {
    let mut p = vec![0u8; 33];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2..4].copy_from_slice(&command.to_le_bytes());
    p[4] = 0; // confirmation
    for (i, v) in params.iter().enumerate() {
        p[5 + i * 4..9 + i * 4].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    encode_v2(sysid, compid, seq, msg::COMMAND_LONG, &p)
}

/// 编码 SET_MODE（ArduPilot：base_mode = CUSTOM_MODE_ENABLED，custom_mode = 模式号）
pub fn encode_set_mode(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    custom_mode: u32,
) -> Vec<u8> {
    let mut p = vec![0u8; 6];
    p[0] = target_sys;
    p[1] = consts::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED;
    p[2..6].copy_from_slice(&custom_mode.to_le_bytes());
    encode_v2(sysid, compid, seq, msg::SET_MODE, &p)
}

/// 编码 REQUEST_DATA_STREAM（请求全数据流，rate Hz）
pub fn encode_request_data_stream(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    rate: u16,
) -> Vec<u8> {
    let mut p = vec![0u8; 6];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2] = consts::MAV_DATA_STREAM_ALL;
    p[3..5].copy_from_slice(&rate.to_le_bytes());
    p[5] = 1; // start
    encode_v2(sysid, compid, seq, msg::REQUEST_DATA_STREAM, &p)
}

/// 编码 SET_POSITION_TARGET_GLOBAL_INT（随点随行：只带位置，忽略速度/加速度/偏航）
///
/// # 参数
/// - `lat_deg` / `lon_deg`：目标坐标（WGS84）
/// - `alt_m`：目标相对高度（米）
pub fn encode_set_position_global(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    lat_deg: f64,
    lon_deg: f64,
    alt_m: f32,
) -> Vec<u8> {
    let mut p = vec![0u8; 53];
    p[0..4].copy_from_slice(&0u32.to_le_bytes()); // time_boot_ms
    p[4] = target_sys;
    p[5] = target_comp;
    p[6] = 6; // MAV_FRAME_GLOBAL_INT
    let mask = mask::VELOCITY_IGNORE | mask::ACCEL_IGNORE | mask::YAW_IGNORE | mask::YAW_RATE_IGNORE;
    p[7..9].copy_from_slice(&mask.to_le_bytes());
    p[9..13].copy_from_slice(&((lat_deg * 1e7).round() as i32).to_le_bytes());
    p[13..17].copy_from_slice(&((lon_deg * 1e7).round() as i32).to_le_bytes());
    p[17..21].copy_from_slice(&alt_m.to_bits().to_le_bytes());
    encode_v2(sysid, compid, seq, msg::SET_POSITION_TARGET_GLOBAL_INT, &p)
}

/// 编码 SET_POSITION_TARGET_LOCAL_NED（键盘/摇杆速度控制，机体坐标系）
///
/// # 参数
/// - `vx` / `vy` / `vz`：机体速度（m/s，向前 / 向右 / 向下）
pub fn encode_set_position_local(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    vx: f32,
    vy: f32,
    vz: f32,
) -> Vec<u8> {
    let mut p = vec![0u8; 53];
    p[0..4].copy_from_slice(&0u32.to_le_bytes()); // time_boot_ms
    p[4] = target_sys;
    p[5] = target_comp;
    p[6] = 1; // MAV_FRAME_BODY_NED
    let mask = mask::POSITION_IGNORE | mask::ACCEL_IGNORE | mask::YAW_IGNORE | mask::YAW_RATE_IGNORE;
    p[7..9].copy_from_slice(&mask.to_le_bytes());
    p[21..25].copy_from_slice(&vx.to_bits().to_le_bytes());
    p[25..29].copy_from_slice(&vy.to_bits().to_le_bytes());
    p[29..33].copy_from_slice(&vz.to_bits().to_le_bytes());
    encode_v2(sysid, compid, seq, msg::SET_POSITION_TARGET_LOCAL_NED, &p)
}

/// 编码 MISSION_COUNT（通知飞控开始接收 n 条任务）
pub fn encode_mission_count(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    count: u16,
) -> Vec<u8> {
    let mut p = vec![0u8; 5];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2..4].copy_from_slice(&count.to_le_bytes());
    p[4] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_COUNT, &p)
}

/// 编码 MISSION_ITEM_INT（任务条目，MAV_FRAME_GLOBAL_RELATIVE_ALT=3）
///
/// # 参数
/// - `command`：MAV_CMD（NAV_WAYPOINT=16 航点 / DO_SET_SERVO / DO_SET_CAM_TRIGG_INTERVAL 动作）
/// - `param1`：悬停时间（秒）/ DO 命令参数 1
/// - `param2`：转弯半径（米，0 直角）/ DO 命令参数 2
/// - `param4`：目标偏航（度）
pub fn encode_mission_item_int(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    item_seq: u16,
    command: u16,
    lat_deg: f64,
    lon_deg: f64,
    alt_m: f32,
    param1: f32,
    param2: f32,
    param4: f32,
) -> Vec<u8> {
    let mut p = vec![0u8; 38];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2..4].copy_from_slice(&item_seq.to_le_bytes());
    p[4] = 3; // MAV_FRAME_GLOBAL_RELATIVE_ALT
    p[5..7].copy_from_slice(&command.to_le_bytes());
    p[7] = 0; // current
    p[8] = 1; // autocontinue
    p[9..13].copy_from_slice(&param1.to_bits().to_le_bytes());
    p[13..17].copy_from_slice(&param2.to_bits().to_le_bytes());
    p[21..25].copy_from_slice(&param4.to_bits().to_le_bytes());
    p[25..29].copy_from_slice(&((lat_deg * 1e7).round() as i32).to_le_bytes());
    p[29..33].copy_from_slice(&((lon_deg * 1e7).round() as i32).to_le_bytes());
    p[33..37].copy_from_slice(&alt_m.to_bits().to_le_bytes());
    p[37] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_ITEM_INT, &p)
}

/// 编码 MISSION_REQUEST_INT（下载：请求指定序号条目）
pub fn encode_mission_request_int(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    item_seq: u16,
) -> Vec<u8> {
    let mut p = vec![0u8; 5];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2..4].copy_from_slice(&item_seq.to_le_bytes());
    p[4] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_REQUEST_INT, &p)
}

/// 编码 MISSION_ACK（任务结果确认）
pub fn encode_mission_ack(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
    result: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 4];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2] = result;
    p[3] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_ACK, &p)
}

/// 编码 MISSION_CLEAR_ALL（清除任务）
pub fn encode_mission_clear_all(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 3];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_CLEAR_ALL, &p)
}

/// 编码 MISSION_REQUEST_LIST（下载：请求任务列表）
pub fn encode_mission_request_list(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 3];
    p[0] = target_sys;
    p[1] = target_comp;
    p[2] = consts::MISSION_TYPE_MISSION;
    encode_v2(sysid, compid, seq, msg::MISSION_REQUEST_LIST, &p)
}

/// 编码 COMMAND_ACK（模拟器回复命令用）
pub fn encode_command_ack(
    sysid: u8,
    compid: u8,
    seq: u8,
    command: u16,
    result: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 10];
    p[0..2].copy_from_slice(&command.to_le_bytes());
    p[2] = result;
    encode_v2(sysid, compid, seq, msg::COMMAND_ACK, &p)
}

/// ArduPilot Copter 自定义模式 → 模式名
///
/// 模式号同时用于任务文件 `MODE` 字段，长期稳定；
/// 未知模式返回 `CUSTOM(n)` 形式。
pub fn mode_name(custom_mode: u32) -> String {
    match custom_mode {
        0 => "STABILIZE",
        1 => "ACRO",
        2 => "ALT_HOLD",
        3 => "AUTO",
        4 => "GUIDED",
        5 => "LOITER",
        6 => "RTL",
        7 => "CIRCLE",
        9 => "LAND",
        11 => "DRIFT",
        13 => "SPORT",
        16 => "POSHOLD",
        17 => "BRAKE",
        18 => "THROW",
        _ => "CUSTOM",
    }
    .to_string()
}

/// ArduPilot Copter 模式名 → 模式号（未知返回 None）
pub fn mode_id(name: &str) -> Option<u32> {
    Some(match name.trim().to_uppercase().as_str() {
        "STABILIZE" => 0,
        "ACRO" => 1,
        "ALT_HOLD" | "ALTHOLD" => 2,
        "AUTO" => 3,
        "GUIDED" => 4,
        "LOITER" => 5,
        "RTL" => 6,
        "CIRCLE" => 7,
        "LAND" => 9,
        "DRIFT" => 11,
        "SPORT" => 13,
        "POSHOLD" => 16,
        "BRAKE" => 17,
        "THROW" => 18,
        // 数字模式 ID（如 "16"）
        _ => name.trim().parse().ok()?,
    })
}

/// 编码 PARAM_REQUEST_LIST（请求飞控发送全部参数）
#[allow(dead_code)]
pub fn encode_param_request_list(
    sysid: u8,
    compid: u8,
    seq: u8,
    target_sys: u8,
    target_comp: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 2];
    p[0] = target_sys;
    p[1] = target_comp;
    encode_v2(sysid, compid, seq, msg::PARAM_REQUEST_LIST, &p)
}

/// PARAM_SET 解码结果（地面站写入参数）
#[derive(Debug, Clone)]
pub struct ParamSet {
    /// 参数名（最多 16 字节 ASCII，左对齐，0 填充）
    pub param_id: String,
    /// 参数值（单精度浮点）
    pub param_value: f32,
}

/// 解码 PARAM_SET（param_id 16 字节 + param_value f32）
pub fn decode_param_set(p: &[u8]) -> ParamSet {
    let n = p.len().min(16);
    let mut id_bytes = [0u8; 16];
    id_bytes[..n].copy_from_slice(&p[..n]);
    let param_id = String::from_utf8_lossy(&id_bytes)
        .trim_end_matches('\0')
        .trim()
        .to_string();
    let param_value = if p.len() >= 20 { rd_f32(p, 16) } else { 0.0 };
    ParamSet { param_id, param_value }
}

/// 编码 PARAM_VALUE（飞控回执单个参数）
///
/// # 参数
/// - `param_id`：参数名（截断/填充至 16 字节）
/// - `param_value`：参数值
/// - `param_count`：参数总数（用于前端进度）
/// - `param_index`：当前参数序号（0 起）
pub fn encode_param_value(
    sysid: u8,
    compid: u8,
    seq: u8,
    param_id: &str,
    param_value: f32,
    param_count: u16,
    param_index: u16,
) -> Vec<u8> {
    let mut p = vec![0u8; 25];
    // param_id：16 字节 ASCII（左对齐，0 填充）
    let id_bytes = param_id.as_bytes();
    let n = id_bytes.len().min(16);
    p[..n].copy_from_slice(&id_bytes[..n]);
    p[16..20].copy_from_slice(&param_value.to_bits().to_le_bytes());
    p[20] = consts::MAV_PARAM_TYPE_REAL32;
    p[21..23].copy_from_slice(&param_count.to_le_bytes());
    p[23..25].copy_from_slice(&param_index.to_le_bytes());
    encode_v2(sysid, compid, seq, msg::PARAM_VALUE, &p)
}

/// 编码 EKF_STATUS_REPORT（36 字节 v2）
///
/// # 参数
/// - `flags`：EKF 各状态位掩码（MAV_ESTIMATOR_STATUS，位 0~15）
/// - `vel_var` / `pos_h_var` / `pos_v_var` / `compass_var`：核心方差（越小越健康）
pub fn encode_ekf_status_report(
    sysid: u8,
    compid: u8,
    seq: u8,
    flags: u32,
    vel_var: f32,
    pos_h_var: f32,
    pos_v_var: f32,
    compass_var: f32,
) -> Vec<u8> {
    let mut p = vec![0u8; 36];
    p[0..4].copy_from_slice(&flags.to_le_bytes());
    p[4..8].copy_from_slice(&vel_var.to_bits().to_le_bytes());
    p[8..12].copy_from_slice(&pos_h_var.to_bits().to_le_bytes());
    p[12..16].copy_from_slice(&pos_v_var.to_bits().to_le_bytes());
    p[16..20].copy_from_slice(&compass_var.to_bits().to_le_bytes());
    // 其余方差字段（terrain_alt / baro/ GPS / wind 等）占位填 0
    encode_v2(sysid, compid, seq, msg::EKF_STATUS_REPORT, &p)
}

/// EKF_STATUS_REPORT 解码结果
#[derive(Debug, Clone)]
pub struct EkfStatus {
    /// 各状态位掩码
    pub flags: u32,
    /// 速度方差
    pub vel_var: f32,
    /// 水平位置方差
    pub pos_h_var: f32,
    /// 垂直位置方差
    pub pos_v_var: f32,
    /// 罗盘方差
    pub compass_var: f32,
}

/// 解码 EKF_STATUS_REPORT
pub fn decode_ekf_status_report(p: &[u8]) -> EkfStatus {
    EkfStatus {
        flags: rd_u32(p, 0),
        vel_var: rd_f32(p, 4),
        pos_h_var: rd_f32(p, 8),
        pos_v_var: rd_f32(p, 12),
        compass_var: rd_f32(p, 16),
    }
}

/// 编码 VIBRATION（28 字节 v2：time_usec + vibration xyz + clipping 3 路）
pub fn encode_vibration(
    sysid: u8,
    compid: u8,
    seq: u8,
    time_usec: u64,
    vx: f32,
    vy: f32,
    vz: f32,
) -> Vec<u8> {
    let mut p = vec![0u8; 28];
    p[0..8].copy_from_slice(&time_usec.to_le_bytes());
    p[8..12].copy_from_slice(&vx.to_bits().to_le_bytes());
    p[12..16].copy_from_slice(&vy.to_bits().to_le_bytes());
    p[16..20].copy_from_slice(&vz.to_bits().to_le_bytes());
    // clipping 3 路占位填 0
    encode_v2(sysid, compid, seq, msg::VIBRATION, &p)
}

/// VIBRATION 解码结果
#[derive(Debug, Clone)]
pub struct Vibration {
    /// 振动 x（m/s/s RMS）
    pub x: f32,
    /// 振动 y
    pub y: f32,
    /// 振动 z
    pub z: f32,
}

/// 解码 VIBRATION（取三轴振动）
pub fn decode_vibration(p: &[u8]) -> Vibration {
    Vibration {
        x: rd_f32(p, 8),
        y: rd_f32(p, 12),
        z: rd_f32(p, 16),
    }
}

/// 编码 RC_CHANNELS（42 字节 v2：time_boot_ms + chan1..18 raw + rssi）
pub fn encode_rc_channels(
    sysid: u8,
    compid: u8,
    seq: u8,
    time_boot_ms: u32,
    channels: &[u16; 18],
    rssi: u8,
) -> Vec<u8> {
    let mut p = vec![0u8; 42];
    p[0..4].copy_from_slice(&time_boot_ms.to_le_bytes());
    for i in 0..18 {
        let off = 4 + i * 2;
        p[off..off + 2].copy_from_slice(&channels[i].to_le_bytes());
    }
    p[40] = rssi;
    encode_v2(sysid, compid, seq, msg::RC_CHANNELS, &p)
}

/// RC_CHANNELS 解码结果
#[derive(Debug, Clone)]
pub struct RcChannels {
    /// 18 路通道原始值（PWM，微秒；未使用通道为 0）
    pub channels: [u16; 18],
    /// 接收机信号强度（0-100，0 表示遥控器丢失）
    pub rssi: u8,
}

/// 解码 RC_CHANNELS
pub fn decode_rc_channels(p: &[u8]) -> RcChannels {
    let mut channels = [0u16; 18];
    for i in 0..18 {
        let off = 4 + i * 2;
        if off + 2 <= p.len() {
            channels[i] = u16::from_le_bytes([p[off], p[off + 1]]);
        }
    }
    let rssi = if p.len() >= 41 { p[40] } else { 0 };
    RcChannels { channels, rssi }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编码 → 提取 → 解码 全链路自洽（含 CRC 校验）
    #[test]
    fn encode_extract_roundtrip() {
        let mut extractor = FrameExtractor::new();
        let frame = encode_command_long(255, 190, 1, 1, 1, cmd::COMPONENT_ARM_DISARM, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let frames = extractor.feed(&frame);
        assert_eq!(frames.len(), 1);
        let f = &frames[0];
        assert_eq!(f.msgid, msg::COMMAND_LONG);
        assert_eq!(f.sysid, 255);
        let decoded = decode_command_long(&f.payload);
        assert_eq!(decoded.command, cmd::COMPONENT_ARM_DISARM);
        assert_eq!(decoded.params[0], 1.0);
    }

    /// 两帧粘连 + 前置垃圾字节的重同步
    #[test]
    fn extractor_resync() {
        let mut extractor = FrameExtractor::new();
        let hb = encode_heartbeat(1, 1, 0, consts::MAV_TYPE_QUADROTOR, consts::MAV_AUTOPILOT_ARDUPILOTMEGA, 0x81, 4, consts::MAV_STATE_ACTIVE);
        let att = encode_v2(1, 1, 1, msg::ATTITUDE, &{
            let mut p = vec![0u8; 28];
            p[4..8].copy_from_slice(&0.1f32.to_bits().to_le_bytes());
            p
        });
        let mut stream = vec![0x00, 0xFF, 0x12];
        stream.extend_from_slice(&hb);
        stream.extend_from_slice(&att);
        let frames = extractor.feed(&stream);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].msgid, msg::HEARTBEAT);
        assert_eq!(frames[1].msgid, msg::ATTITUDE);
        let h = decode_heartbeat(&frames[0].payload);
        assert_eq!(h.custom_mode, 4);
        assert!(h.base_mode & consts::MAV_MODE_FLAG_SAFETY_ARMED != 0);
    }

    /// 分片喂入
    #[test]
    fn extractor_fragmented() {
        let mut extractor = FrameExtractor::new();
        let frame = encode_v2(1, 1, 0, msg::VFR_HUD, &[1.5f32.to_bits().to_le_bytes(), 2.5f32.to_bits().to_le_bytes()].concat());
        let mut frames = extractor.feed(&frame[..5]);
        assert!(frames.is_empty());
        frames = extractor.feed(&frame[5..]);
        assert_eq!(frames.len(), 1);
        let vfr = decode_vfr_hud(&frames[0].payload);
        assert!((vfr.airspeed - 1.5).abs() < 1e-4);
        assert!((vfr.groundspeed - 2.5).abs() < 1e-4);
    }
}
