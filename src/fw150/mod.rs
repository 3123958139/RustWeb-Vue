//! fw150 角色模块 —— 由 `role_template` 复制而来（验证角色解耦）。
//!
//! 角色：fw150（设备台账）
//! - 权限点：`Permission::Fw150Monitor`（已在 `src/roles.rs` 注册表授予）
//! - 路由：`/api/fw150`（仅拥有 Fw150Monitor 权限的角色可访问）

#![allow(dead_code)] // 模板模块默认未挂载，避免 dead_code 警告

pub mod handlers;
pub mod routes;
pub mod services;
