//! city3d 角色专有实现（二级目录）
//!
//! 一级目录 `src/city3d/` 仅保留模板骨架（mod.rs / handlers.rs / routes.rs / services.rs），
//! 本目录承载全部角色专有子模块，由一级 mod.rs `pub use` 再导出，
//! 外部仍以 `crate::city3d::x` 路径访问。

pub mod models;
