//! # FTJ1C 模块全局状态
//!
//! 存放服务运行标志、配置文件路径、共享 QuadFrame 与停止信号。
//!
//! ## 设计原则
//! - 使用 `AtomicBool` 实现多线程无锁读写，避免互斥锁开销
//! - 使用 `OnceLock` 实现惰性初始化，确保线程安全的单次创建
//! - 使用 `Arc` 包装共享数据，支持跨线程所有权转移
//!
//! ## 线程安全
//! - `SERVICE_RUNNING`: 服务运行状态，多线程读写
//! - `STOP_SIGNAL`: 停止信号，服务线程轮询，主线程设置
//! - `QUAD_FRAME`: 共享帧缓冲区，UDP 线程写入，串口线程/前端读取

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use crate::ftj1c::quad_frame::QuadFrame;

/// 服务运行标志（多线程无锁读写）
///
/// # 说明
/// - `true`: 服务已启动，正在运行
/// - `false`: 服务未启动或已停止
///
/// # 线程安全
/// 使用 `AtomicBool` + `Relaxed` 内存序，适用于状态标志场景。
pub static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// 配置文件路径（相对进程工作目录，ftj1c 角色独立配置文件）
///
/// # 说明
/// 固定为 `config-ftj1c.ini`，与进程工作目录拼接得到完整路径。
/// 此常量在 `service.rs` 和 `handlers.rs` 中使用。
pub const CONFIG_PATH: &str = "config-ftj1c.ini";

/// 全局停止信号（服务线程轮询该标志退出）
///
/// # 说明
/// 使用 `OnceLock<Arc<AtomicBool>>` 实现惰性初始化。
/// 服务线程在主循环中轮询此标志，发现为 `true` 时退出。
static STOP_SIGNAL: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// 全局共享 QuadFrame（主备双链写入，单路发送/前端读取）
///
/// # 说明
/// 使用 `OnceLock<Arc<QuadFrame>>` 实现惰性初始化。
/// `Arc` 允许多个线程共享所有权，无需移动或克隆数据。
static QUAD_FRAME: OnceLock<Arc<QuadFrame>> = OnceLock::new();

/// 获取全局停止信号
///
/// # 返回值
/// `Arc<AtomicBool>` 的克隆，可跨线程传递和共享。
///
/// # 说明
/// 首次访问时自动创建 `AtomicBool`（初始值 `false`）。
/// 后续调用返回同一实例的克隆。
pub fn stop_signal() -> Arc<AtomicBool> {
    STOP_SIGNAL
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// 复位停止信号（服务启动时调用）
///
/// # 说明
/// 将停止信号重置为 `false`，允许服务重新启动。
/// 在 `service::start_service` 中调用，确保新服务可以正常运行。
pub fn reset_stop_signal() {
    stop_signal().store(false, std::sync::atomic::Ordering::Relaxed);
}

/// 获取全局 QuadFrame
///
/// # 返回值
/// `Arc<QuadFrame>` 的克隆，可跨线程传递和共享。
///
/// # 说明
/// 首次访问时自动创建 `QuadFrame` 实例（4 个清零槽位，默认主链）。
/// 后续调用返回同一实例的克隆。
pub fn quad_frame() -> Arc<QuadFrame> {
    QUAD_FRAME
        .get_or_init(|| Arc::new(QuadFrame::new()))
        .clone()
}
