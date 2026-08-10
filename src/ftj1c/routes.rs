//! # FTJ1C 通信监控模块路由（ftj1c 角色域，挂载于 /api/ftj1c）
//!
//! HTTP 端点需要认证（Bearer Token）；WebSocket 通过 `?token=` 参数在
//! handler 内鉴权（浏览器 WS 无法携带自定义头），因此 /ws 不挂中间件。
//! 事件通道使用 `crate::ftj1c::ftj1c_tx()` 全局广播，路由仅依赖数据库连接。
//!
//! ## 路由结构
//!
//! ```text
//! /api/ftj1c/
//! ├── /service/start    (POST)  启动服务
//! ├── /service/stop     (POST)  停止服务
//! ├── /service/status   (GET)   查询状态
//! ├── /ip-config        (GET)   获取 IP 配置
//! ├── /config           (GET)   读取配置文件
//! │                   (PUT)   保存配置文件
//! └── /ws               (GET)   WebSocket 连接
//! ```
//!
//! ## 中间件层级
//!
//! ```text
//! 请求 → auth_middleware（JWT 验证） → ftj1c_permission_middleware（权限检查） → handler
//! ```
//! WebSocket 端点绕过中间件，在 handler 内部通过 `?token=` 参数鉴权。

use crate::common::middleware::permission_middleware;
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::ftj1c::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post, put},
    Router,
};

/// 创建 FTJ1C 通信监控模块的路由树
///
/// # 参数
/// - `db`: 数据库连接，作为应用状态传递给中间件和 handler
///
/// # 返回值
/// 配置好的 `Router<DatabaseConnection>`，挂载于 `/api/ftj1c`
///
/// # 路由规则
/// - HTTP 端点：需要 JWT 认证 + `ftj1c:monitor` 权限
/// - WebSocket 端点：handler 内部通过查询参数 `?token=` 鉴权
pub fn ftj1c_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let auth = middleware::from_fn_with_state(db.clone(), crate::common::middleware::auth_middleware);

    // 受 Bearer 认证保护的端点
    let protected = Router::<DatabaseConnection>::new()
        .route("/service/start", post(handlers::start_service_handler))
        .route("/service/stop", post(handlers::stop_service_handler))
        .route("/service/status", get(handlers::service_status_handler))
        .route("/ip-config", get(handlers::ip_config_handler))
        .route("/config", get(handlers::read_config_handler))
        .route("/config", put(handlers::save_config_handler))
        .route_layer(middleware::from_fn_with_state(
            db.clone(),
            ftj1c_permission_middleware,
        ))
        .route_layer(auth);

    // WebSocket：handler 内部用 ?token= 校验 JWT
    Router::<DatabaseConnection>::new()
        .route("/ws", get(handlers::ws_handler))
        .merge(protected)
}

/// FTJ1C 模块权限中间件
///
/// # 功能
/// 检查用户是否拥有 `ftj1c:monitor` 权限（登录 + 权限校验）
///
/// # 参数
/// - `db`: 数据库连接
/// - `request`: HTTP 请求
/// - `next`: 下一个中间件或 handler
async fn ftj1c_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::Ftj1cMonitor, State(db), request, next).await
}
