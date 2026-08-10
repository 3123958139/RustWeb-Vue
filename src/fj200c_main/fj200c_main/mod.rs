//! fj200c_main 角色专有实现（二级目录）
//!
//! 一级目录 `src/fj200c_main/` 仅保留模板骨架（mod.rs / handlers.rs / routes.rs / service.rs），
//! 本目录承载全部角色专有子模块，由一级 mod.rs `pub use` 再导出，
//! 外部仍以 `crate::fj200c_main::x` 路径访问。

pub mod abstract_com;
pub mod com;
pub mod config;
pub mod decode;
pub mod mock;
pub mod report;
pub mod state;
pub mod types;
