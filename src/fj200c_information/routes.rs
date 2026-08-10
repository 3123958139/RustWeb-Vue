//! # 发动机监控模块路由（fj200c_information 角色域，挂载于 /api/fj200c_information）
//!
//! HTTP 端点需要认证（Bearer Token）；WebSocket 通过 `?token=` 参数在
//! handler 内鉴权（浏览器 WS 无法携带自定义头），因此 /ws 不挂中间件。
//! 事件通道使用 `crate::fj200c_information::fj200c_information_tx()` 全局广播，路由仅依赖数据库连接。
//!
//! ## 路由结构
//!
//! ```text
//! /api/fj200c_information
//!   ├── /service/start    POST   启动服务
//!   ├── /service/stop     POST   停止服务
//!   ├── /service/status   GET    查询状态
//!   ├── /service/command  POST   发送命令
//!   ├── /config           GET    读取配置
//!   ├── /config           PUT    保存配置
//!   ├── /csv/files        GET    列出 CSV 文件
//!   ├── /csv/:name        GET    读取 CSV 文件内容
//!   └── /ws               GET    WebSocket（handler 内鉴权）
//! ```
//!
//! ## 中间件链
//!
//! 1. `auth_middleware`：验证 JWT Bearer Token（从 Authorization 头提取）
//! 2. `fj200c_information_permission_middleware`：校验 `fj200c_information:monitor` 权限
//! 3. WebSocket 路由不经过上述中间件，在 `ws_handler` 内部用 `?token=` 校验

use crate::common::middleware::permission_middleware;
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::fj200c_information::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post, put},
    Router,
};

/// 构建发动机监控模块的子路由树
///
/// - `db`：数据库连接池，注入为路由状态供中间件和 handler 使用
/// - 返回挂载在 `/api/fj200c_information` 下的完整路由
pub fn fj200c_information_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let auth = middleware::from_fn_with_state(db.clone(), crate::common::middleware::auth_middleware);

    // 受 Bearer 认证 + fj200c_information:monitor 权限保护的端点
    let protected = Router::<DatabaseConnection>::new()
        .route("/service/start", post(handlers::start_service_handler))
        .route("/service/stop", post(handlers::stop_service_handler))
        .route("/service/status", get(handlers::service_status_handler))
        .route("/service/command", post(handlers::send_command_handler))
        .route("/config", get(handlers::read_config_handler))
        .route("/config", put(handlers::save_config_handler))
        .route("/csv/files", get(handlers::list_csv_files_handler))
        .route("/csv/:name", get(handlers::get_csv_file_handler))
        // 中间件执行顺序：从外到内，先权限后认证
        .route_layer(middleware::from_fn_with_state(
            db.clone(),
            fj200c_information_permission_middleware,
        ))
        .route_layer(auth);

    // WebSocket：handler 内部用 ?token= 校验 JWT，不走 Bearer 中间件
    Router::<DatabaseConnection>::new()
        .route("/ws", get(handlers::ws_handler))
        .merge(protected)
}

/// fj200c_information 权限中间件：要求登录且拥有 `fj200c_information:monitor` 权限
///
/// 委托给通用的 `permission_middleware`，传入 `Permission::Fj200cInformationMonitor` 枚举值。
async fn fj200c_information_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::Fj200cInformationMonitor, State(db), request, next).await
}
