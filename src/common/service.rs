//! # 公共服务运行时（线程句柄 / 停止标志管理）
//!
//! 各角色服务（fj200c_information / ftj1c）共用的启动/停止编排基础设施：
//! - 工作线程句柄的存储与 join
//! - 「停止进行中」标志（防启动/停止竞态）
//! - 异步停止骨架 `stop_in_background`（置停止信号 → 后台 join → 复位标志）
//!
//! 各角色仍各自维护运行状态（`SERVICE_RUNNING`）与停止信号，
//! 因为停止信号的具体形态不同（fj200c_information 用 `AtomicBool` 全局量，
//! ftj1c 用 `OnceLock<Arc<AtomicBool>>` 惰性单例）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// 服务运行时：线程句柄集合 + 停止进行中标志
pub struct ServiceRuntime {
    /// 工作线程句柄存储（停止时依次 join，确保线程干净退出）
    handles: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>>,
    /// 停止进行中的标志位，防止在停止流程中再次启动服务
    stopping: AtomicBool,
}

impl ServiceRuntime {
    /// 创建空的运行时实例
    pub const fn new() -> Self {
        Self {
            handles: OnceLock::new(),
            stopping: AtomicBool::new(false),
        }
    }

    /// 获取线程句柄存储的内部引用（惰性初始化）
    fn handles(&self) -> &Mutex<Vec<thread::JoinHandle<()>>> {
        self.handles.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// 登记一个工作线程句柄
    pub fn push(&self, handle: thread::JoinHandle<()>) {
        self.handles().lock().unwrap_or_else(|e| e.into_inner()).push(handle);
    }

    /// 取出并清空所有线程句柄（用于停止时 join）
    pub fn drain(&self) -> Vec<thread::JoinHandle<()>> {
        self.handles().lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect()
    }

    /// 设置「停止进行中」标志
    pub fn set_stopping(&self, value: bool) {
        self.stopping.store(value, Ordering::Relaxed);
    }

    /// 是否正在停止流程中
    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Relaxed)
    }

    /// 等待停止流程结束（最多等待 timeout），供启动服务时防竞态
    pub fn wait_stopping(&self, timeout: Duration) {
        let deadline = std::time::Instant::now() + timeout;
        while self.is_stopping() && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// 异步停止骨架：置停止标志后，在独立线程中 join 所有工作线程并复位状态
///
/// 停止流程（不阻塞调用方，HTTP 请求立即返回）：
/// 1. 置「停止进行中」标志 + 调用 `set_stop` 触发工作线程退出
/// 2. 独立线程中依次 `join` 所有句柄，确保线程干净退出
/// 3. 复位 `running`（`false`）与「停止进行中」标志
///
/// # 参数
/// - `runtime` / `running`：`static` 生命周期（角色模块的 `static RUNTIME` / `SERVICE_RUNNING`）
/// - `set_stop`：置停止信号的闭包（各角色停止信号形态不同，如 `STOP_SIGNAL.store(true)`）
/// - `log_msg`：停止完成后的日志消息
pub fn stop_in_background(
    runtime: &'static ServiceRuntime,
    running: &'static AtomicBool,
    set_stop: impl FnOnce() + Send + 'static,
    log_msg: &'static str,
) {
    runtime.set_stopping(true);
    set_stop();
    thread::spawn(move || {
        for handle in runtime.drain() {
            let _ = handle.join();
        }
        running.store(false, Ordering::Relaxed);
        runtime.set_stopping(false);
        tracing::info!("{}", log_msg);
    });
}

/// 按角色隔离停止后台线程与资源（公共组件，所有角色通用）
///
/// 保证**有且只有当前角色保持线程与资源**：
///
/// - `keep_role = None`：停止全部三个角色服务（退出登录场景，无当前角色）
/// - `keep_role = Some(role)`：仅停止**其他角色**自有的服务线程与资源，
///   当前角色（`role`）的服务保持运行（切换角色 / 切换账号 / 启动当前角色服务场景）
///
/// 各角色停止函数幂等（未运行则直接返回），可随时安全调用：
///
/// - `fj200c_information`：串口/模拟采集会话线程、CSV 写线程
/// - `fj200c_main`：五路串口（ECU/ADAM/DYNO/Flux）读线程、周期发送线程、
///   处理线程、模拟发送线程，并关闭 CSV 录制文件
/// - `ftj1c`：UDP 组播接收线程、串口发送线程
/// - `qgc`：UDP 接收/发送线程、模拟飞控线程
pub fn stop_all_services_except(keep_role: Option<&str>) {
    if keep_role != Some("fj200c_information") {
        crate::fj200c_information::service::stop_service();
    }
    if keep_role != Some("fj200c_main") {
        crate::fj200c_main::service::stop_all();
    }
    if keep_role != Some("ftj1c") {
        crate::ftj1c::service::stop_service();
    }
    if keep_role != Some("qgc") {
        crate::qgc::service::stop_service();
    }
    tracing::info!("角色服务线程清理完成（保留: {:?}）", keep_role);
}
