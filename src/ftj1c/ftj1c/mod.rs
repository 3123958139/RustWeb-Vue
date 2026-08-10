//! ftj1c 角色专有实现（二级目录）
//!
//! 一级目录 `src/ftj1c/` 仅保留模板骨架（mod.rs / handlers.rs / routes.rs / service.rs），
//! 本目录承载全部角色专有子模块，由一级 mod.rs `pub use` 再导出，
//! 外部仍以 `crate::ftj1c::x` 路径访问。

pub mod com;
pub mod config;
pub mod models;
pub mod process;
pub mod quad_frame;
pub mod state;
pub mod udp;
