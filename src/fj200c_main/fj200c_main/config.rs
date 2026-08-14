//! 全局配置单例（`config-fj200c_main.ini`）
//!
//! 配置以 `Option<Config>` 形式存于全局 `OnceLock<RwLock>`，读取方通过
//! `global()` 获取读锁（返回 None 表示尚未加载），服务启动/重启时通过
//! `set_global` 重新加载配置（见 `fj200c_main/service.rs` 的重启逻辑）。
use std::sync::{OnceLock, RwLock, RwLockReadGuard};

pub use crate::common::config::Config;

/// 全局配置槽：`None` 表示配置尚未加载
static GLOBAL: OnceLock<RwLock<Option<Config>>> = OnceLock::new();

/// 获取全局配置锁（惰性初始化单例）
fn lock() -> &'static RwLock<Option<Config>> {
    GLOBAL.get_or_init(|| RwLock::new(None))
}

/// 读取当前配置（未加载时返回 None）
pub fn global() -> Option<RwLockReadGuard<'static, Option<Config>>> {
    lock().read().ok()
}

/// 写入/替换当前配置（服务启动或重启时调用）
pub fn set_global(cfg: Config) {
    if let Ok(mut guard) = lock().write() {
        *guard = Some(cfg);
    }
}

/// 清空配置（服务停止时调用，避免持有过期配置）
pub fn clear_global() {
    if let Ok(mut guard) = lock().write() {
        *guard = None;
    }
}
