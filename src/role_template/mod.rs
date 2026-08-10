//! 角色模块模板 —— 新增角色的标准骨架（完整可编译，默认不挂载）。
//!
//! 启用步骤（后端侧）：
//! 1. 在 `src/roles.rs` 的 `ROLE_REGISTRY` 中注册新角色（key / name / 权限点）；
//! 2. 复制本目录为 `src/<新角色名>/`（或按需改名），实现业务 handler / service；
//! 3. 在 `src/main.rs` 声明 `mod <新角色名>;`；
//! 4. 在 `src/routes.rs` 中调用 `<新角色名>::routes::xxx_router(db)` 挂载路由，
//!    并选择中间件：
//!    - 用户面板功能：`permission_middleware(新权限点, ...)` 或仅 `auth_middleware`
//!    - 管理功能：`role_middleware`（注册表中授予 SystemAdmin 权限即生效）
//! 5. 前端侧：见 `packages/shared/src/roles.ts` 与 `packages/shared/src/template/TemplatePanel.vue`。
//!
//! 新增权限点链路：`src/common/models.rs` 的 Permission 枚举
//! → `src/roles.rs` 注册表授予 → 前端 `packages/shared/src/types.ts` 同步。

#![allow(dead_code)] // 模板模块默认未挂载，避免 dead_code 警告

pub mod handlers;
pub mod routes;
pub mod services;
