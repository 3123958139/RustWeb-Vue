//! fw100 角色模块路由：设备台账查询（权限点 fw100:monitor）。
//! 挂载于 `/api/fw100`，见 `src/routes.rs`。

use crate::common::middleware::{auth_middleware, permission_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::fw100::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::get,
    Router,
};

pub fn fw100_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let protected = Router::<DatabaseConnection>::new()
        .route("/items", get(handlers::list_fw100_items))
        .route_layer(middleware::from_fn_with_state(db.clone(), fw100_permission_middleware))
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));

    Router::<DatabaseConnection>::new().merge(protected)
}

// 权限中间件：登录 + 拥有 fw150:read 权限
async fn fw100_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::Fw100Monitor, State(db), request, next).await
}
