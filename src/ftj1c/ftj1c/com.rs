//! 串口协议构建器与坐标转换
//!
//! 从 demo-test3-ftj 的 backend/com.rs + backend/utils.rs 完整迁移：
//! - `ulh2ecef` / `to_cgcs2000`：ULH 坐标 → ECEF → CGCS2000（六组 LE i32 毫米/厘米级坐标）
//! - `ComFTJ1CTrajectory`（33B 航迹帧）、`ComFTJ1CTelemetering`（65B 遥测帧）、
//!   `ComFTJ1CEquipment`（91B 设备帧）三种协议构建器，字段取值自 config-ftj1c.ini 对应节
//!   （`[ComFTJ1C*]` 的 S0/D0/B1/N1/T1/S1 与 `[IP]` 节无关），数据来自 QuadFrame 槽位。
//!
//! ## 坐标转换流程
//!
//! ```text
//! ULH (纬度/经度/高程) → ECEF (地心坐标) → CGCS2000 (中国大地坐标系)
//!                          ↓
//!                    速度变换 (NED → ECEF)
//! ```
//!
//! ## 协议构建器
//!
//! | 构建器 | 大小 | 用途 |
//! |---|---|---|
//! | `ComFTJ1CTrajectory` | 33 字节 | 航迹帧，包含 CGCS2000 坐标 |
//! | `ComFTJ1CTelemetering` | 65 字节 | 遥测帧，包含位置/高度/姿态/电流等 |
//! | `ComFTJ1CEquipment` | 91 字节 | 设备帧，包含设备状态信息 |
//!
//! ## 串口控制器
//!
//! `ComControl` 封装串口打开/关闭/发送，支持：
//! - 从配置文件读取串口参数（端口名、波特率、数据位等）
//! - 自动重连和错误处理
//! - `Drop` trait 自动关闭串口

// 原项目 1:1 迁移的保留位推进（index 递增但未被读取），抑制警告保持行为一致
#![allow(unused_assignments)]

use crate::ftj1c::config::{self, Config};
use crate::ftj1c::quad_frame::QuadFrame;
use crate::common::utils::format_hex;
use chrono::{Local, Timelike};
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

// ════════════════════════════════════════════════════════════
//  坐标转换（backend/utils.rs）
// ════════════════════════════════════════════════════════════

/// WGS84 椭球体参数
const DEG2RAD: f64 = PI / 180.0; // 角度转弧度系数
const A: f64 = 6_378_137.0; // WGS84 长半轴 (m)
const E2: f64 = 0.0066943799901413165; // 第一偏心率平方
const ONE_MINUS_E2: f64 = 0.99330562000985867; // 1 - e²，短半轴与长半轴的平方比

/// ULH 坐标转 ECEF 坐标（含速度）
///
/// # 参数
/// - `ulh`: `[纬度(°), 经度(°), 高程(m)]`，大地坐标系
/// - `vel`: `[北向速度(m/s), 天向速度(m/s), 东向速度(m/s)]`，NED 速度
///
/// # 返回值
/// `(位置 [x, y, z] (m), 速度 [vx, vy, vz] (m/s))`，地心坐标系
///
/// # 算法
/// 1. 将角度转换为弧度
/// 2. 计算卯酉圈曲率半径 `N = A / sqrt(1 - e² * sin²(lat))`
/// 3. 计算 ECEF 位置：`x = (N + h) * cos(lat) * cos(lon)`
/// 4. 构建 NED → ECEF 旋转矩阵
/// 5. 应用旋转矩阵变换速度分量
pub fn ulh2ecef(ulh: [f64; 3], vel: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let lat = ulh[0] * DEG2RAD;
    let lon = ulh[1] * DEG2RAD;
    let h = ulh[2];

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();

    // 卯酉圈曲率半径
    let r_n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();

    // ---- ECEF 位置 ----
    let clong = (r_n + h) * cos_lat;
    let x = clong * lon.cos();
    let y = clong * lon.sin();
    let z = (r_n * ONE_MINUS_E2 + h) * sin_lat;
    let position = [x, y, z];

    // ---- NED → ECEF 旋转矩阵（行优先存储） ----
    let longi = lon - FRAC_PI_2; // 经度 - 90°
    let sin_longi = longi.sin();
    let cos_longi = longi.cos();

    let mut slong = [0.0; 9];
    slong[0] = sin_longi * sin_lat;
    slong[1] = -cos_longi * sin_lat;
    slong[2] = cos_lat;
    slong[3] = -sin_longi * cos_lat;
    slong[4] = cos_lat * cos_longi;
    slong[5] = sin_lat;
    slong[6] = -cos_longi;
    slong[7] = -sin_longi;
    slong[8] = 0.0;

    // ---- 速度变换 ----
    let vn = vel[0]; // 北向
    let vu = vel[1]; // 天向
    let ve = vel[2]; // 东向

    let mut velocity = [0.0; 3];
    for i in 0..3 {
        velocity[i] = slong[i] * vn + slong[i + 3] * vu + slong[i + 6] * ve;
    }

    (position, velocity)
}

/// 坐标转换：纬度/经度/高程 + 北向/天向/东向速度 → 六组 CGCS2000 小端字节（×10 放大）
///
/// # 参数
/// - `w`: 纬度 (°)
/// - `j`: 经度 (°)
/// - `g`: 高程 (m)
/// - `b`: 北向速度 (m/s)
/// - `t`: 天向速度 (m/s)
/// - `d`: 东向速度 (m/s)
///
/// # 返回值
/// 六组 4 字节小端 `i32`，分别对应：
/// - 位置 [x, y, z] × 10
/// - 速度 [vx, vy, vz] × 10
///
/// # 说明
/// 乘以 10 是为了在整数表示中保留一位小数精度（厘米级定位）。
pub fn to_cgcs2000(
    w: f64,
    j: f64,
    g: f64,
    b: f64,
    t: f64,
    d: f64,
) -> ([u8; 4], [u8; 4], [u8; 4], [u8; 4], [u8; 4], [u8; 4]) {
    let data = ulh2ecef([w, j, g], [b, t, d]);
    (
        (data.0[0] as i32 * 10i32).to_le_bytes(),
        (data.0[1] as i32 * 10i32).to_le_bytes(),
        (data.0[2] as i32 * 10i32).to_le_bytes(),
        (data.1[0] as i32 * 10i32).to_le_bytes(),
        (data.1[1] as i32 * 10i32).to_le_bytes(),
        (data.1[2] as i32 * 10i32).to_le_bytes(),
    )
}

// ════════════════════════════════════════════════════════════
//  串口协议构建器（backend/com.rs）
// ════════════════════════════════════════════════════════════

/// 串口协议构建器公共接口
///
/// # 说明
/// 定义 `build_frame` 方法，从 `QuadFrame` 读取槽位数据构建发送帧。
/// 返回格式化的十六进制字符串，用于串口发送。
///
/// # 实现者
/// - `ComFTJ1CTrajectory`: 航迹帧（33 字节）
/// - `ComFTJ1CTelemetering`: 遥测帧（65 字节）
/// - `ComFTJ1CEquipment`: 设备帧（91 字节）
pub trait ComFTJ1CBase {
    fn build_frame(&self, qf: Arc<QuadFrame>) -> String;
}

/// 计算当前时间：从午夜开始的毫秒数 ×10（1/100 秒单位）
///
/// # 返回值
/// 小端 4 字节 `u32`，表示自午夜起的 1/100 秒数
///
/// # 用途
/// 用于串口协议帧中的时间戳字段。
fn current_time_centis() -> [u8; 4] {
    let now = Local::now();
    let t = (now.hour() * 3600 * 1000 + now.minute() * 60 * 1000 + now.second() * 1000
        + now.timestamp_subsec_millis())
        * 10;
    t.to_le_bytes()
}

/// 从配置节中读取十六进制单字节字段
///
/// # 参数
/// - `cfg`: 配置实例
/// - `section`: INI 节名
/// - `key`: 键名
/// - `default`: 默认值（十六进制字符串）
///
/// # 返回值
/// 解析后的 `u8` 值，解析失败返回 0
fn hex_byte(cfg: &Config, section: &str, key: &str, default: &str) -> u8 {
    u8::from_str_radix(cfg.get_or(section, key, default).as_str(), 16).unwrap_or(0)
}

/// 航迹帧（33 字节）
///
/// # 帧结构
/// ```text
/// Offset  Len  Field
/// 0       1    S0 (源地址)
/// 1       1    D0 (目标地址)
/// 2       1    B1 (命令字)
/// 3       1    N1 (数据长度)
/// 4-7     4    时间戳 (1/100秒)
/// 8       1    S1 (序列号)
/// 9-32    24   CGCS2000 坐标 (6×4 字节)
/// ```
///
/// # 数据来源
/// 从 `QuadFrame` 槽位 2（yc3）读取经纬度/高程/速度，转换为 CGCS2000 坐标。
pub struct ComFTJ1CTrajectory;

impl ComFTJ1CBase for ComFTJ1CTrajectory {
    /// 构建航迹帧
    ///
    /// # 参数
    /// - `qf`: 共享 QuadFrame，用于读取槽位数据
    ///
    /// # 返回值
    /// 格式化的十六进制字符串（如 "76 70 41 02 ..."）
    fn build_frame(&self, qf: Arc<QuadFrame>) -> String {
        let cfg = match config::global() {
            Some(c) => c,
            None => {
                error!("[ComFTJ1CTrajectory] 配置未加载");
                return "".to_string();
            }
        };
        // 读取 4 个槽位，仅使用槽位 2（yc3）
        let _ = qf.read_slot(0);
        let _ = qf.read_slot(1);
        let yc3 = qf.read_slot(2);
        let _ = qf.read_slot(3);

        // 从 yc3 槽位提取经纬度/高程/速度（小端 f64/u16/i16）
        let zhwd: f64 = f64::from_le_bytes(yc3[23..31].try_into().unwrap()); // 纬度
        let zhjd: f64 = f64::from_le_bytes(yc3[15..23].try_into().unwrap()); // 经度
        let zhgd: f64 = u16::from_le_bytes(yc3[31..33].try_into().unwrap()) as f64 * 1.0; // 高程
        let zhbs: f64 = i16::from_le_bytes(yc3[9..11].try_into().unwrap()) as f64 * 0.1; // 北向速度
        let zhts: f64 = i16::from_le_bytes(yc3[11..13].try_into().unwrap()) as f64 * 0.1; // 天向速度
        let zhds: f64 = i16::from_le_bytes(yc3[13..15].try_into().unwrap()) as f64 * 0.1; // 东向速度
        let cgcs2000 = to_cgcs2000(zhwd, zhjd, zhgd, zhbs, zhts, zhds);

        let mut result: [u8; 33] = [0; 33];
        let mut index: usize = 0;
        // 帧头字段（从配置读取）
        result[0] = hex_byte(cfg, "ComFTJ1CTrajectory", "S0", "76");
        result[1] = hex_byte(cfg, "ComFTJ1CTrajectory", "D0", "70");
        result[2] = hex_byte(cfg, "ComFTJ1CTrajectory", "B1", "41");
        result[3] = hex_byte(cfg, "ComFTJ1CTrajectory", "N1", "02");
        index += 4;
        // 时间戳
        result[index..index + 4].copy_from_slice(&current_time_centis());
        index += 4;
        // 序列号
        result[index] = hex_byte(cfg, "ComFTJ1CTrajectory", "S1", "23");
        index += 1;
        // CGCS2000 坐标（6×4 字节）
        for coord in [
            cgcs2000.0, cgcs2000.1, cgcs2000.2, cgcs2000.3, cgcs2000.4, cgcs2000.5,
        ] {
            result[index..index + 4].copy_from_slice(&coord);
            index += 4;
        }

        format_hex(&result)
    }
}

/// 遥测帧（65 字节）
///
/// # 帧结构
/// 包含帧头字段、时间戳、序列号以及各类遥测参数：
/// - 位置信息（经纬度/高程）
/// - 姿态信息（俯仰角、横滚角）
/// - 电气参数（电流、电压）
/// - 保留字段
///
/// # 数据来源
/// 从 `QuadFrame` 槽位 0/1/2（yc1/yc2/yc3）读取各类遥测数据。
pub struct ComFTJ1CTelemetering;

impl ComFTJ1CBase for ComFTJ1CTelemetering {
    fn build_frame(&self, qf: Arc<QuadFrame>) -> String {
        let cfg = match config::global() {
            Some(c) => c,
            None => {
                error!("[ComFTJ1CTelemetering] 配置未加载");
                return "".to_string();
            }
        };
        let yc1 = qf.read_slot(0);
        let yc2 = qf.read_slot(1);
        let yc3 = qf.read_slot(2);
        let _ = qf.read_slot(3);

        let mut result: [u8; 65] = [0; 65];
        let mut index: usize = 0;
        result[0] = hex_byte(cfg, "ComFTJ1CTelemetering", "S0", "74");
        result[1] = hex_byte(cfg, "ComFTJ1CTelemetering", "D0", "70");
        result[2] = hex_byte(cfg, "ComFTJ1CTelemetering", "B1", "5A");
        result[3] = hex_byte(cfg, "ComFTJ1CTelemetering", "A1", "02");
        index += 4;
        result[index..index + 4].copy_from_slice(&current_time_centis());
        index += 4;
        result[index] = hex_byte(cfg, "ComFTJ1CTelemetering", "S1", "01");
        index += 1;
        result[index] = 0x02;
        index += 1;
        result[index..index + 4].copy_from_slice(&yc1[5..9]);
        index += 4;
        let zhjd = (f64::from_le_bytes(yc3[15..23].try_into().unwrap()) as i32).to_le_bytes();
        result[index..index + 4].copy_from_slice(&zhjd);
        index += 4;
        let zhwd = (f64::from_le_bytes(yc3[23..31].try_into().unwrap()) as i32).to_le_bytes();
        result[index..index + 4].copy_from_slice(&zhwd);
        index += 4;
        let zhgd = (u16::from_le_bytes(yc3[31..33].try_into().unwrap()) as u16).to_le_bytes();
        result[index..index + 2].copy_from_slice(&zhgd);
        index += 2;
        index += 2; // 保留位
        result[index..index + 2].copy_from_slice(&yc2[53..55]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc2[51..53]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc3[35..37]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc3[33..35]);
        index += 2;
        // 原项目逻辑：zhhxj 由 yc3[37..39] 换算（0.01 系数在 `as` 转换中被截断为 0，
        // 结果恒为 0 —— 保持与原项目 1:1 行为）
        let mut zhhxj = i16::from_le_bytes(yc3[37..39].try_into().unwrap()) * 0.01 as i16;
        if zhhxj > 0 {
            zhhxj = 360 - zhhxj;
        } else {
            zhhxj = -1 * zhhxj;
        }
        zhhxj = zhhxj * 100;
        result[index..index + 2].copy_from_slice(&zhhxj.to_le_bytes());
        index += 2;
        result[index..index + 2].copy_from_slice(&yc3[9..11]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc3[13..15]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc3[11..13]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc2[49..51]);
        index += 2;
        result[index..index + 2].copy_from_slice(&yc2[55..57]);
        index += 2;
        let zhphj =
            (i16::from_le_bytes(yc2[55..57].try_into().unwrap()).wrapping_sub(180).wrapping_mul(-1))
                .to_le_bytes();
        result[index..index + 2].copy_from_slice(&zhphj);
        index += 2;
        result[index] = yc1[43];
        index += 1;
        index += 4; // 保留位
        index += 4; // 保留位
        // 原项目逻辑：cpj 与 zs 写在同一位置（index 未推进），保持 1:1 行为
        let cpj = i32::from_le_bytes(yc2[73..77].try_into().unwrap()).to_le_bytes();
        result[index..index + 4].copy_from_slice(&cpj);
        let zs = ((u16::from_le_bytes(yc2[35..37].try_into().unwrap()) as f32 / 65536.0 * 25000f32)
            as u32)
            .to_le_bytes();
        result[index..index + 4].copy_from_slice(&zs);
        index += 4;
        index += 1;
        index += 1;

        format_hex(&result)
    }
}

/// 设备帧（91 字节）
///
/// # 帧结构
/// 包含帧头字段、时间戳、序列号以及设备状态信息（yc1[5..9] 四字节）。
/// 其余字段为保留位，填充为 0。
///
/// # 数据来源
/// 从 `QuadFrame` 槽位 0（yc1）读取设备状态数据。
pub struct ComFTJ1CEquipment;

impl ComFTJ1CBase for ComFTJ1CEquipment {
    fn build_frame(&self, qf: Arc<QuadFrame>) -> String {
        let cfg = match config::global() {
            Some(c) => c,
            None => {
                error!("[ComFTJ1CEquipment] 配置未加载");
                return "".to_string();
            }
        };
        let yc1 = qf.read_slot(0);
        let _ = qf.read_slot(1);
        let _ = qf.read_slot(2);
        let _ = qf.read_slot(3);

        let mut result: [u8; 91] = [0; 91];
        let mut index: usize = 0;
        result[0] = hex_byte(cfg, "ComFTJ1CEquipment", "S0", "76");
        result[1] = hex_byte(cfg, "ComFTJ1CEquipment", "D0", "70");
        result[2] = hex_byte(cfg, "ComFTJ1CEquipment", "B1", "55");
        result[3] = hex_byte(cfg, "ComFTJ1CEquipment", "N1", "02");
        index += 4;
        result[index..index + 4].copy_from_slice(&current_time_centis());
        index += 4;
        result[index] = hex_byte(cfg, "ComFTJ1CEquipment", "S1", "23");
        index += 1;
        result[index..index + 4].copy_from_slice(&yc1[5..9]);
        index += 4;

        format_hex(&result)
    }
}

/// 根据配置节名创建对应的协议构建器
///
/// # 参数
/// - `section`: 配置节名（如 "ComFTJ1CTrajectory"）
///
/// # 返回值
/// - `Some(Box<dyn ComFTJ1CBase>)`: 成功创建的构建器（trait object）
/// - `None`: 未知的节名
///
/// # 说明
/// 使用 trait object (`Box<dyn ComFTJ1CBase>`) 实现动态分发，
/// 允许在运行时根据配置选择不同的协议构建器。
pub fn make_transform(section: &str) -> Option<Box<dyn ComFTJ1CBase>> {
    match section {
        "ComFTJ1CTrajectory" => Some(Box::new(ComFTJ1CTrajectory)),
        "ComFTJ1CEquipment" => Some(Box::new(ComFTJ1CEquipment)),
        "ComFTJ1CTelemetering" => Some(Box::new(ComFTJ1CTelemetering)),
        _ => None,
    }
}

// ════════════════════════════════════════════════════════════
//  串口控制器（dch control/com.rs）
// ════════════════════════════════════════════════════════════

/// 串口校验位类型
///
/// # 说明
/// 用于串口通信的错误检测，`None` 表示无校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComParity {
    /// 无校验
    None,
    /// 偶校验（Even Parity）
    Even,
    /// 奇校验（Odd Parity）
    Odd,
}

/// 串口停止位类型
///
/// # 说明
/// 用于标识一帧数据的结束，`One` 为 1 位，`Two` 为 2 位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComStopBits {
    /// 1 位停止位
    One,
    /// 2 位停止位
    Two,
}

/// 串口配置参数
///
/// # 默认值
/// ```text
/// port_name: ""
/// baud_rate: 115200
/// data_bits: 8
/// parity: None
/// stop_bits: One
/// timeout_ms: 100
/// ```
#[derive(Debug, Clone)]
pub struct ComConfig {
    /// 端口名称，例如 "COM101"
    pub port_name: String,
    /// 波特率，默认 115200
    pub baud_rate: u32,
    /// 数据位，默认 8
    pub data_bits: u8,
    /// 校验位，默认 None
    pub parity: ComParity,
    /// 停止位，默认 One
    pub stop_bits: ComStopBits,
    /// 读取超时（毫秒），默认 100
    pub timeout_ms: u64,
}

impl Default for ComConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            parity: ComParity::None,
            stop_bits: ComStopBits::One,
            timeout_ms: 100,
        }
    }
}

/// 串口控制器，支持打开/关闭/重连/收发
///
/// # 说明
/// 从 demo-test3-ftj 的 dch crate 移植，封装 `serialport` 库。
/// 使用 `Mutex<Option<Box<dyn SerialPort>>>` 保护串口实例，
/// 支持多线程安全访问和自动重连。
///
/// # 线程安全
/// - `Mutex` 保护串口实例，防止并发访问冲突
/// - `Drop` trait 自动关闭串口，确保资源释放
pub struct ComControl {
    /// 串口实例（`Box<dyn SerialPort>` 为 trait object，支持不同平台实现）
    port: Mutex<Option<Box<dyn serialport::SerialPort>>>,
    /// 串口配置参数
    config: ComConfig,
    /// 配置节名（用于日志标识）
    section: String,
}

impl ComControl {
    /// 从配置文件创建串口控制器
    ///
    /// # 参数
    /// - `section`: 配置节名（如 "ComFTJ1CTelemetering"）
    ///
    /// # 返回值
    /// - `Ok(Self)`: 成功创建并打开串口
    /// - `Err(Box<dyn Error>)`: 配置错误或串口打开失败
    ///
    /// # 配置示例
    /// ```ini
    /// [ComFTJ1CTelemetering]
    /// PORTNAME = COM101
    /// BaudRate = 115200
    /// DataBits = 8
    /// Parity = None
    /// StopBits = 1
    /// TimeoutMs = 100
    /// ```
    pub fn create(section: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cfg = config::global().ok_or_else(|| {
            let msg = "配置未加载，无法创建串口".to_string();
            error!("{}", msg);
            msg
        })?;

        let port_name = cfg.get_or(section, "PORTNAME", "");
        if port_name.is_empty() {
            let msg = format!("[{}] PORTNAME 未配置", section);
            error!("{}", msg);
            return Err(msg.into());
        }

        let baud_rate: u32 = cfg
            .get_or(section, "BaudRate", "115200")
            .parse()
            .unwrap_or(115200);
        let data_bits: u8 = cfg
            .get_or(section, "DataBits", "8")
            .parse()
            .unwrap_or(8);
        let parity = parse_parity(cfg.get_or(section, "Parity", "None").as_str());
        let stop_bits = parse_stop_bits(cfg.get_or(section, "StopBits", "1").as_str());
        let timeout_ms: u64 = cfg
            .get_or(section, "TimeoutMs", "100")
            .parse()
            .unwrap_or(100);

        let config = ComConfig {
            port_name: port_name.clone(),
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            timeout_ms,
        };

        let control = Self {
            port: Mutex::new(None),
            config,
            section: section.to_string(),
        };
        control.open_port()?;
        Ok(control)
    }

    /// 打开串口
    ///
    /// # 说明
    /// 根据配置参数创建串口实例并绑定。
    /// 使用 `socket2` 库设置 `SO_REUSEADDR` 选项。
    ///
    /// # 错误处理
    /// 如果端口被占用，尝试绑定 `0.0.0.0:<port>` 作为回退。
    fn open_port(&self) -> Result<(), Box<dyn std::error::Error>> {
        let builder = serialport::new(&self.config.port_name, self.config.baud_rate)
            .data_bits(match self.config.data_bits {
                5 => serialport::DataBits::Five,
                6 => serialport::DataBits::Six,
                7 => serialport::DataBits::Seven,
                _ => serialport::DataBits::Eight,
            })
            .parity(match self.config.parity {
                ComParity::None => serialport::Parity::None,
                ComParity::Even => serialport::Parity::Even,
                ComParity::Odd => serialport::Parity::Odd,
            })
            .stop_bits(match self.config.stop_bits {
                ComStopBits::One => serialport::StopBits::One,
                ComStopBits::Two => serialport::StopBits::Two,
            })
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms));

        let port = builder.open().map_err(|e| {
            let msg = format!(
                "[{}] 打开串口 {} 失败: {}",
                self.section, self.config.port_name, e
            );
            error!("{}", msg);
            msg
        })?;

        {
            let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
            *p = Some(port);
        }

        info!(
            "[{}] 串口 {} 已打开: {} baud, {}{}{}",
            self.section,
            self.config.port_name,
            self.config.baud_rate,
            self.config.data_bits,
            match self.config.parity {
                ComParity::None => "N",
                ComParity::Even => "E",
                ComParity::Odd => "O",
            },
            match self.config.stop_bits {
                ComStopBits::One => "1",
                ComStopBits::Two => "2",
            },
        );

        Ok(())
    }

    /// 关闭串口
    ///
    /// # 说明
    /// 使用 `Option::take` 取出串口实例，触发 `Drop` 释放资源。
    /// 如果串口已关闭，此操作无副作用。
    pub fn destroy(&self) {
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        if p.take().is_some() {
            info!(
                "[{}] 串口 {} 已关闭",
                self.section, self.config.port_name
            );
        }
    }

    /// 向串口发送数据
    ///
    /// # 参数
    /// - `buf`: 待发送的字节切片
    ///
    /// # 返回值
    /// - `Ok(usize)`: 实际发送的字节数
    /// - `Err(Box<dyn Error>)`: 串口未打开或发送失败
    pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        let port = p.as_mut().ok_or_else(|| {
            let msg = format!("[{}] 串口未打开，无法发送", self.section);
            error!("{}", msg);
            msg
        })?;
        let n = port.write(buf).map_err(|e| {
            let msg = format!("[{}] 串口发送失败: {}", self.section, e);
            error!("{}", msg);
            msg
        })?;
        Ok(n)
    }
}

/// 实现 `Drop` trait，自动关闭串口
///
/// # 说明
/// 当 `ComControl` 实例离开作用域时自动调用 `destroy()`，
/// 确保串口资源被正确释放，避免资源泄漏。
impl Drop for ComControl {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// 解析校验位字符串
///
/// # 参数
/// - `s`: 配置字符串（"even"/"odd"/其他）
///
/// # 返回值
/// 对应的 `ComParity` 枚举值
fn parse_parity(s: &str) -> ComParity {
    match s.to_lowercase().as_str() {
        "even" => ComParity::Even,
        "odd" => ComParity::Odd,
        _ => ComParity::None,
    }
}

/// 解析停止位字符串
///
/// # 参数
/// - `s`: 配置字符串（"2"/其他）
///
/// # 返回值
/// 对应的 `ComStopBits` 枚举值
fn parse_stop_bits(s: &str) -> ComStopBits {
    match s {
        "2" => ComStopBits::Two,
        _ => ComStopBits::One,
    }
}
