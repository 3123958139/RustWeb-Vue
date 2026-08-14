//! # 认证路由模块
//!
//! 定义认证相关的路由和中间件保护。
//!
//! # 路由表
//!
//! | 路径 | 方法 | 处理器 | 保护 |
//! |------|------|--------|------|
//! | `/api/auth/login` | POST | `handlers::login` | 无 |
//! | `/api/auth/profile` | GET | `handlers::get_profile` | `auth_middleware` |
//! | `/api/auth/logout` | POST | `handlers::logout` | `auth_middleware` |
//!
//! # 路由分组
//!
//! ```text
//! /api/auth/
//! ├── POST /login        （公开，无需登录）
//! ├── GET  /profile      （需要登录）
//! └── POST /logout       （需要登录，停止所有角色后台线程与资源）
//! ```
//!
//! # 语法说明
//!
//! - `Router::new()`: 创建空路由
//! - `.route(path, method_router)`: 添加路由
//! - `.layer(middleware)`: 添加中间件层
//! - `.merge(router)`: 合并两个路由

use crate::common::auth::handlers;           // 认证处理器
use crate::common::middleware::auth_middleware; // 认证中间件
use crate::database::DatabaseConnection;     // 数据库连接池
use axum::{middleware, routing::get, routing::post, Router}; // 路由相关

/// 创建认证路由
///
/// # 参数
/// - `db`: 数据库连接池
///
/// # 返回值
/// 配置好的认证路由
///
/// # 设计说明
///
/// 将需要保护的路由（`/profile`）和不需要保护的路由（`/login`）分开：
/// - `/login` 是公开端点，任何人都可以访问
/// - `/profile` 需要登录，通过 `auth_middleware` 保护
///
/// 这种设计避免了登录端点被中间件拦截的死锁问题。
pub fn auth_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    // 创建受保护的路由组
    // `route_layer` 为当前路由组的所有路由添加中间件
    let protected = Router::<DatabaseConnection>::new()
        .route("/profile", get(handlers::get_profile))  // GET /api/auth/profile
        .route("/logout", post(handlers::logout))       // POST /api/auth/logout
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));  // 添加认证中间件

    // 创建公开路由 + 受保护路由
    Router::new()
        .route("/login", post(handlers::login))  // POST /api/auth/login（公开）
        .merge(protected)                        // 合并受保护的路由
}
