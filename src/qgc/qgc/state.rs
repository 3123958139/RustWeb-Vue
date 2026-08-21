//! # QGC 模块全局状态
//!
//! 存放服务运行标志、配置文件路径、停止信号、遥测快照、
//! 任务状态与下行命令通道。
//!
//! ## 设计原则
//! - `AtomicBool`：服务运行标志 / 停止信号，多线程无锁读写
//! - `OnceLock<Arc<...>>`：遥测快照（`RwLock`）、任务状态（`Mutex`）惰性单例
//! - `OnceLock<Mutex<Option<mpsc::Sender>>>`：下行命令通道（服务停止后置空）

use crate::qgc::mission::MissionState;
use crate::qgc::models::QgcMissionItem;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

/// 服务运行标志（多线程无锁读写）
pub static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// 配置文件路径（相对进程工作目录，qgc 角色独立配置文件）
pub const CONFIG_PATH: &str = "config-qgc.ini";

/// 全局停止信号（服务线程轮询该标志退出）
static STOP_SIGNAL: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// 遥测快照（接收线程更新，WS 初始推送 / GET /telemetry 读取）
static TELEMETRY: OnceLock<Arc<RwLock<crate::qgc::models::QgcTelemetry>>> = OnceLock::new();

/// 任务状态（接收线程 / 发送线程 / handler 共享）
static MISSION: OnceLock<Arc<Mutex<MissionState>>> = OnceLock::new();

/// 下行命令通道（HTTP handler 写入，发送线程消费；服务停止后置 None）
static OUTBOUND: OnceLock<Mutex<Option<mpsc::Sender<Outbound>>>> = OnceLock::new();

/// 下行通道消息
#[derive(Debug)]
pub enum Outbound {
    /// 已编码的 MAVLink v2 帧（命令 / 模式切换）
    Frame(Vec<u8>),
    /// 任务上传（携带航点列表）
    MissionUpload(Vec<QgcMissionItem>),
    /// 任务下载请求
    MissionDownload,
    /// 任务清除请求
    MissionClear,
}

/// 获取全局停止信号（惰性初始化）
pub fn stop_signal() -> Arc<AtomicBool> {
    STOP_SIGNAL
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// 复位停止信号（服务启动时调用）
pub fn reset_stop_signal() {
    stop_signal().store(false, std::sync::atomic::Ordering::Relaxed);
}

/// 获取遥测快照（服务未启动时为默认快照）
pub fn telemetry() -> Arc<RwLock<crate::qgc::models::QgcTelemetry>> {
    TELEMETRY
        .get_or_init(|| Arc::new(RwLock::new(crate::qgc::models::QgcTelemetry::default())))
        .clone()
}

/// 重置遥测快照为默认值（服务启动时清除旧连接数据）
pub fn reset_telemetry() {
    *telemetry().write().unwrap_or_else(|e| e.into_inner()) =
        crate::qgc::models::QgcTelemetry::default();
}

/// 获取任务状态
pub fn mission() -> Arc<Mutex<MissionState>> {
    MISSION.get_or_init(|| Arc::new(Mutex::new(MissionState::new()))).clone()
}

/// 重置任务状态（服务启动/停止时）
pub fn reset_mission() {
    *mission().lock().unwrap_or_else(|e| e.into_inner()) = MissionState::new();
}

/// 登记下行命令通道（服务启动时调用，内部消费由发送线程持有）
pub fn set_outbound(tx: mpsc::Sender<Outbound>) {
    *OUTBOUND
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(tx);
}

/// 清除下行命令通道（服务停止时调用）
pub fn clear_outbound() {
    if let Some(lock) = OUTBOUND.get() {
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// 获取下行命令通道（服务未启动时返回 None）
pub fn outbound_sender() -> Option<mpsc::Sender<Outbound>> {
    OUTBOUND
        .get()
        .and_then(|lock| lock.lock().unwrap_or_else(|e| e.into_inner()).clone())
}

/// 下行帧序号（原子递增，保证同一次会话内 SEQ 不重复）
pub fn next_seq() -> u8 {
    use std::sync::atomic::Ordering;
    static SEQ: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// 待回执命令（发送后等待 `COMMAND_ACK`，超时自动重传）
///
/// 命令（COMMAND_LONG）发出后存入此单例；收到对应 `COMMAND_ACK` 时清除；
/// 发送线程每 100ms 检查，超过 `ACK_TIMEOUT` 未回执则重传，最多 `ACK_MAX_RETRIES` 次，
/// 避免飞控丢包导致命令「石沉大海」。
#[derive(Clone)]
pub struct PendingCommand {
    /// MAV_CMD 命令码
    pub command: u16,
    /// COMMAND_LONG 的 7 个参数
    pub params: [f32; 7],
    /// 最近一次发送时刻（用于超时判定）
    pub sent_at: Instant,
    /// 已重传次数（达到上限后放弃并清除）
    pub retries: u8,
}

/// 命令回执超时（3 秒未收到 COMMAND_ACK 即重传）
pub const ACK_TIMEOUT: Duration = Duration::from_secs(3);
/// 命令最大重传次数（超过后放弃，避免无限刷包）
pub const ACK_MAX_RETRIES: u8 = 3;

static PENDING_CMD: OnceLock<Mutex<Option<PendingCommand>>> = OnceLock::new();

/// 登记待回执命令（命令发出时调用）
pub fn set_pending(command: u16, params: [f32; 7]) {
    let lock = PENDING_CMD.get_or_init(|| Mutex::new(None));
    *lock.lock().unwrap_or_else(|e| e.into_inner()) = Some(PendingCommand {
        command,
        params,
        sent_at: Instant::now(),
        retries: 0,
    });
}

/// 清除待回执命令（收到匹配 COMMAND_ACK 时调用）
pub fn clear_pending(command: u16) {
    if let Some(lock) = PENDING_CMD.get() {
        let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = g.as_ref() {
            if p.command == command {
                *g = None;
            }
        }
    }
}

/// 取超时待重传命令（内部递增重试计数；达上限返回 None 并清除）
pub fn take_expired_pending() -> Option<PendingCommand> {
    let lock = PENDING_CMD.get_or_init(|| Mutex::new(None));
    let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
    let expired = match g.as_mut() {
        Some(p) => p.sent_at.elapsed() >= ACK_TIMEOUT,
        None => false,
    };
    if expired {
        let p = g.as_mut().unwrap();
        if p.retries >= ACK_MAX_RETRIES {
            *g = None;
            return None;
        }
        p.retries += 1;
        p.sent_at = Instant::now();
        return Some(p.clone());
    }
    None
}

/// 参数表（模拟器维护，HTTP 快照读取；PARAM_SET 经飞控更新后回填）
static PARAM_TABLE: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();

/// 默认参数表（ArduCopter 精简子集，覆盖系统/导航/姿态/电池/失控保护）
fn default_params() -> HashMap<String, f32> {
    let mut m = HashMap::new();
    m.insert("SYSID_THISMAV".to_string(), 1.0);
    m.insert("RTL_ALT".to_string(), 15.0);
    m.insert("WPNAV_SPEED".to_string(), 5.0);
    m.insert("LOIT_SPEED".to_string(), 5.0);
    m.insert("PSC_VELXY_P".to_string(), 1.0);
    m.insert("ATC_RAT_RLL_P".to_string(), 0.15);
    m.insert("ATC_RAT_PIT_P".to_string(), 0.15);
    m.insert("ATC_RAT_YAW_P".to_string(), 0.40);
    m.insert("BATT_ARM_VOLT".to_string(), 10.5);
    m.insert("FS_THR_ENABLE".to_string(), 1.0);
    m
}

/// 获取参数表全局句柄（惰性初始化为默认值）
pub fn param_table() -> &'static Mutex<HashMap<String, f32>> {
    PARAM_TABLE.get_or_init(|| Mutex::new(default_params()))
}

/// 重置参数表为默认（服务启动时调用，避免跨会话残留）
pub fn reset_param_table() {
    let lock = param_table();
    let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
    *g = default_params();
}

/// 写入单个参数（PARAM_SET 经飞控处理后回填；HTTP 写入亦可直达表）
pub fn set_param(name: &str, value: f32) -> bool {
    let lock = param_table();
    let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
    if g.contains_key(name) {
        g.insert(name.to_string(), value);
        true
    } else {
        false
    }
}

/// 遥测频率（Hz，运行时可调；模拟器主循环按此频率生成帧）
static TELEMETRY_HZ: OnceLock<Mutex<u16>> = OnceLock::new();

/// 初始化遥测频率（服务启动时按配置 `[Gcs] TelemetryHz` 设置）
pub fn init_telemetry_hz(hz: u16) {
    let lock = TELEMETRY_HZ.get_or_init(|| Mutex::new(hz.max(1)));
    *lock.lock().unwrap_or_else(|e| e.into_inner()) = hz.max(1);
}

/// 设置遥测频率（运行时调速，模拟器下一拍生效，限幅 1~50Hz）
pub fn set_telemetry_hz(hz: u16) {
    let lock = TELEMETRY_HZ.get_or_init(|| Mutex::new(10));
    *lock.lock().unwrap_or_else(|e| e.into_inner()) = hz.clamp(1, 50);
}

/// 读取当前遥测频率
pub fn telemetry_hz() -> u16 {
    *TELEMETRY_HZ
        .get_or_init(|| Mutex::new(10))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
