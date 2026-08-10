//! # 管理员路由模块
//!
//! 定义用户管理路由和双层中间件保护。
//!
//! # 路由结构
//!
//! ```text
//! /api/users
//! ├── GET    /                  → 用户列表（UsersRead + SystemAdmin）
//! ├── POST   /                  → 创建用户（UsersWrite + SystemAdmin）
//! ├── PUT    /:id/role          → 修改角色（UsersWrite + SystemAdmin）
//! ├── DELETE /:id               → 删除用户（UsersDelete + SystemAdmin）
//! └── GET/PUT /settings/pwd-route → 初始密码查询停用开关（SystemAdmin）
//! ```
//!
//! # 双层中间件
//!
//! 每个路由组都经过两层中间件保护：
//! 1. `role_middleware`: 检查 SystemAdmin 权限（管理角色门槛）
//! 2. `users_*_middleware`: 检查具体操作权限（细粒度控制）
//!
//! # 语法说明
//!
//! - `route_layer(middleware)`: 为当前路由组的所有路由添加中间件
//! - 中间件按添加顺序执行（先添加的先执行）
//! - `db.clone()` 克隆连接池（引用计数，成本很低）

use crate::admin::handlers;
use crate::common::middleware::{permission_middleware, role_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{delete, get, post, put},
    Router,
};

/// 创建管理员路由
///
/// # 参数
/// - `db`: 数据库连接池
///
/// # 返回值
/// 配置好的管理员路由
pub fn admin_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    // 读取路由：GET /api/users（用户列表）
    // 中间件顺序：role_middleware → users_read_middleware
    let read_routes = Router::<DatabaseConnection>::new()
        .route("/", get(handlers::list_users))
        .route_layer(middleware::from_fn_with_state(db.clone(), users_read_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), role_middleware));

    // 写入路由：POST /api/users、PUT /api/users/:id/role
    // 中间件顺序：role_middleware → users_write_middleware
    let write_routes = Router::<DatabaseConnection>::new()
        .route("/", post(handlers::create_user))
        .route("/:id/role", put(handlers::update_user_role))
        .route_layer(middleware::from_fn_with_state(db.clone(), users_write_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), role_middleware));

    // 删除路由：DELETE /api/users/:id
    // 中间件顺序：role_middleware → users_delete_middleware
    let delete_routes = Router::<DatabaseConnection>::new()
        .route("/:id", delete(handlers::delete_user))
        .route_layer(middleware::from_fn_with_state(db.clone(), users_delete_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), role_middleware));

    // 系统设置路由：GET/PUT /api/users/settings/pwd-route
    // 管理员控制 GET /admin/pwd 初始密码查询的停用开关（仅 SystemAdmin 门槛）
    let settings_routes = Router::<DatabaseConnection>::new()
        .route(
            "/settings/pwd-route",
            get(handlers::get_pwd_route_status).put(handlers::set_pwd_route_status),
        )
        .route_layer(middleware::from_fn_with_state(db, role_middleware));

    // 合并所有路由组
    read_routes
        .merge(write_routes)
        .merge(delete_routes)
        .merge(settings_routes)
}

/// UsersRead 权限中间件
///
/// # 说明
///
/// 包装 `permission_middleware`，固定检查 `UsersRead` 权限。
/// Axum 的 `middleware::from_fn` 要求函数签名固定，
/// 不能直接传递参数，所以需要这种包装函数。
async fn users_read_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::UsersRead, State(db), request, next).await
}

/// UsersWrite 权限中间件
async fn users_write_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::UsersWrite, State(db), request, next).await
}

/// UsersDelete 权限中间件
async fn users_delete_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::UsersDelete, State(db), request, next).await
}
