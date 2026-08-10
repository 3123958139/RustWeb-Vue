//! # 公共服务运行时（线程句柄 / 停止标志管理）
//!
//! 各角色服务（fj200c_information / ftj1c）共用的启动/停止编排基础设施：
//! - 工作线程句柄的存储与 join
//! - 「停止进行中」标志（防启动/停止竞态）
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
