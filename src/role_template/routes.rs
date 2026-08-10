//! 角色模块模板路由示例。
//! 将 `role_template` 复制为新角色模块后：
//! 1. 把嵌套路径前缀 `/template` 改为新角色的路由前缀；
//! 2. 把 `role_template_permission_middleware` 中占位的权限点换成注册表授予的权限；
//! 3. 在 `src/routes.rs` 中挂载本路由。

use crate::common::middleware::{auth_middleware, permission_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::role_template::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::get,
    Router,
};

pub fn role_template_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let protected = Router::<DatabaseConnection>::new()
        .route("/items", get(handlers::list_template_items))
        .route_layer(middleware::from_fn_with_state(db.clone(), role_template_permission_middleware))
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));

    Router::<DatabaseConnection>::new().nest("/template", protected)
}

// 模板权限中间件：示例为"登录 + 拥有指定权限"。新角色替换占位权限即可。
async fn role_template_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::Fj200cInformationMonitor, State(db), request, next).await
}
