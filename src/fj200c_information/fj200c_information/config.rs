//! # fj200c_information 配置文件管理
//!
//! 复用公共 INI 配置封装（`crate::common::config::Config`），
//! 维护本角色独立的全局实例（config-fj200c_information.ini）。
//!
//! `global()` / `set_global()` 是本模块的独立函数（不是 `Config` 的
//! 关联函数），因为多个角色对同一公共类型实现同名方法会冲突。

use std::sync::OnceLock;

pub use crate::common::config::Config;

/// 全局配置实例（惰性初始化，只读）
static GLOBAL: OnceLock<Config> = OnceLock::new();

/// 获取全局配置实例的只读引用
///
/// 仅在 `set_global` 成功调用后返回 `Some`，否则返回 `None`。
pub fn global() -> Option<&'static Config> {
    GLOBAL.get()
}

/// 设置全局配置实例（只能成功调用一次）
///
/// 第二次调用会返回 `Err(cfg)`，将配置所有权交还给调用者。
pub fn set_global(cfg: Config) -> Result<(), Config> {
    GLOBAL.set(cfg)
}
