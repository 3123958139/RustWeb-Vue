//! # ftj1c 配置文件管理
//!
//! 复用公共 INI 配置封装（`crate::common::config::Config`），
//! 维护本角色独立的全局实例（config-ftj1c.ini，`[Udp]` / `[IP]` / 串口协议节）。
//!
//! `global()` / `set_global()` 是本模块的独立函数（不是 `Config` 的
//! 关联函数），因为多个角色对同一公共类型实现同名方法会冲突。

use std::sync::OnceLock;

pub use crate::common::config::Config;

/// 全局配置实例（惰性初始化，只读）
static GLOBAL: OnceLock<Config> = OnceLock::new();

/// 获取全局配置实例的引用
///
/// - `Some(&'static Config)`: 全局配置已初始化
/// - `None`: 全局配置未初始化（服务未启动）
pub fn global() -> Option<&'static Config> {
    GLOBAL.get()
}

/// 设置全局配置实例（只能成功调用一次）
///
/// - `Ok(())`: 设置成功
/// - `Err(Config)`: 全局配置已初始化（不允许重复设置）
pub fn set_global(cfg: Config) -> Result<(), Config> {
    GLOBAL.set(cfg)
}
