//! # 管理员角色模块
//!
//! 基于 `src/role_template/` 模板派生，功能：用户增查改删 + 角色绑定。
//!
//! # 权限保护（双层中间件）
//!
//! ```text
//! 请求 → role_middleware（检查 SystemAdmin）→ permission_middleware（检查具体权限）→ 处理器
//! ```
//!
//! | 端点 | 方法 | 权限 | 说明 |
//! |------|------|------|------|
//! | `/api/users` | GET | UsersRead | 用户列表 |
//! | `/api/users` | POST | UsersWrite | 创建用户 |
//! | `/api/users/:id/role` | PUT | UsersWrite | 修改角色 |
//! | `/api/users/:id` | DELETE | UsersDelete | 删除用户 |
//!
//! # 安全约束
//!
//! - 角色绑定白名单：只允许注册表中已登记的角色
//! - 不能移除自己的管理角色（防止管理域无人可用）
//! - 不能删除当前登录用户

pub mod handlers;  // 处理器：HTTP 端点实现
pub mod routes;    // 路由：定义路由表和中间件
pub mod services;  // 服务：数据库操作
