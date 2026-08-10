//! city3d 角色模块路由：城市 3D 数字孪生 API（权限点 city3d:view）。
//! 挂载于 `/api/city3d`，见 `src/routes.rs`。

use crate::common::middleware::{auth_middleware, permission_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::city3d::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post, put, delete},
    Router,
};

pub fn city3d_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let protected = Router::<DatabaseConnection>::new()
        // 建筑 CRUD
        .route("/buildings", get(handlers::list_buildings))
        .route("/buildings", post(handlers::create_building))
        .route("/buildings/:id", put(handlers::update_building))
        .route("/buildings/:id", delete(handlers::delete_building))
        // 区域 CRUD
        .route("/districts", get(handlers::list_districts))
        .route("/districts", post(handlers::create_district))
        .route("/districts/:id", put(handlers::update_district))
        .route("/districts/:id", delete(handlers::delete_district))
        // 事件 CRUD
        .route("/events", get(handlers::list_events))
        .route("/events", post(handlers::create_event))
        .route("/events/:id", delete(handlers::delete_event))
        // 概览统计
        .route("/overview", get(handlers::overview))
        .route_layer(middleware::from_fn_with_state(db.clone(), city3d_permission_middleware))
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));

    Router::<DatabaseConnection>::new().merge(protected)
}

/// 权限中间件：登录 + 拥有 city3d:view 权限
async fn city3d_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::City3dView, State(db), request, next).await
}