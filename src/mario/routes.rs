//! mario 角色模块路由：超级马里奥游戏成绩 API（权限点 mario:monitor）。
//! 挂载于 `/api/mario`，见 `src/routes.rs`。

use crate::common::middleware::{auth_middleware, permission_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::mario::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post},
    Router,
};

pub fn mario_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let protected = Router::<DatabaseConnection>::new()
        // 高分榜
        .route("/scores", get(handlers::list_scores))
        // 提交成绩
        .route("/scores", post(handlers::submit_score))
        // 全局统计
        .route("/stats", get(handlers::get_stats))
        .route_layer(middleware::from_fn_with_state(db.clone(), mario_permission_middleware))
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));

    Router::<DatabaseConnection>::new().nest("/mario", protected)
}

/// 权限中间件：登录 + 拥有 mario:monitor 权限
async fn mario_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::MarioMonitor, State(db), request, next).await
}