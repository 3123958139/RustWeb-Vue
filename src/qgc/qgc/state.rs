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
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

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
