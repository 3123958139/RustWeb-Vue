//! protocol_generator 角色模块路由。
//! 挂载于 `/api/protocol_generator`，见 `src/routes.rs`。

use crate::common::middleware::{auth_middleware, permission_middleware};
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::protocol_generator::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post},
    Router,
};

pub fn protocol_generator_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let protected = Router::<DatabaseConnection>::new()
        .route("/default-csv", get(handlers::get_default_csv).put(handlers::save_default_csv))
        .route("/markdown", post(handlers::export_markdown))
        .route("/excel", post(handlers::export_excel))
        .route("/csv/parse", post(handlers::parse_csv))
        .route("/csv/serialize", post(handlers::serialize_csv))
        .route_layer(middleware::from_fn_with_state(db.clone(), protocol_generator_permission_middleware))
        .route_layer(middleware::from_fn_with_state(db, auth_middleware));

    Router::<DatabaseConnection>::new().merge(protected)
}

// 权限中间件：登录 + 拥有 ProtocolGeneratorMonitor 权限
async fn protocol_generator_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::ProtocolGeneratorMonitor, State(db), request, next).await
}