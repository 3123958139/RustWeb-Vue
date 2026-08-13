//! # ftj1c 配置文件管理
//!
//! 复用公共 INI 配置封装（`crate::common::config::Config`），
//! 维护本角色独立的全局实例（config-ftj1c.ini，`[Udp]` / `[IP]` / 串口协议节）。
//!
//! 单例由公共宏 `config_singleton!` 生成（`OnceLock<Config>` 只读）。

pub use crate::common::config::Config;

crate::config_singleton!(GLOBAL, global, set_global);
