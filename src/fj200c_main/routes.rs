//! # FJ-200C 测控模块路由（fj200c_main 角色域，挂载于 /api/fj200c_main）
//!
//! HTTP 端点需要认证（Bearer Token）+ `Fj200cMainMonitor` 权限；
//! WebSocket 通过 `?token=` 参数在 handler 内鉴权（浏览器 WS 无法携带自定义头），
//! 因此 `/ws` 不挂中间件。
//!
//! ## 路由结构
//!
//! ```text
//! /api/fj200c_main/
//! ├── /service/start      (POST)   启动服务
//! ├── /service/stop       (POST)   停止服务
//! ├── /service/status     (GET)    查询状态
//! ├── /service/command    (POST)   发送 ECU 指令
//! ├── /config             (GET)    读取配置文件
//! │                     (PUT)    保存配置文件
//! ├── /csv/files          (GET)    列出 CSV 文件
//! ├── /csv/{name}         (GET)    读取 CSV 文件内容
//! ├── /recording/toggle   (POST)   切换 CSV 录制
//! ├── /simulation/toggle  (POST)   切换模拟运行
//! ├── /theme/set          (POST)   设置主题
//! ├── /experiment         (GET)    获取试验信息
//! │                     (PUT)    保存试验信息
//! ├── /report             (POST)   生成报表
//! ├── /help               (GET)    获取帮助文档
//! └── /ws                 (GET)    WebSocket 连接
//! ```
//!
//! ## 中间件链
//!
//! ```text
//! 请求 → auth_middleware（JWT 验证） → fj200c_main_permission_middleware（权限检查） → handler
//! ```
//! WebSocket 端点绕过中间件，在 handler 内部通过 `?token=` 参数鉴权。

use crate::common::middleware::permission_middleware;
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::fj200c_main::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post, put},
    Router,
};

/// 创建 FJ-200C 测控模块的路由树
///
/// # 参数
/// - `db`: 数据库连接，作为应用状态传递给中间件和 handler
///
/// # 返回值
/// 配置好的 `Router<DatabaseConnection>`，挂载于 `/api/fj200c_main`
///
/// # 路由规则
/// - HTTP 端点：需要 JWT 认证 + `Fj200cMainMonitor` 权限
/// - WebSocket 端点：handler 内部通过查询参数 `?token=` 鉴权
pub fn fj200c_main_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let auth = middleware::from_fn_with_state(db.clone(), crate::common::middleware::auth_middleware);

    // 受 Bearer 认证 + Fj200cMainMonitor 权限保护的端点
    let protected = Router::<DatabaseConnection>::new()
        // 服务控制
        .route("/service/start", post(handlers::start_service_handler))
        .route("/service/stop", post(handlers::stop_service_handler))
        .route("/service/status", get(handlers::service_status_handler))
        .route("/service/command", post(handlers::send_command_handler))
        // 配置文件
        .route("/config", get(handlers::read_config_handler))
        .route("/config", put(handlers::save_config_handler))
        // CSV 数据浏览
        .route("/csv/files", get(handlers::list_csv_files_handler))
        .route("/csv/:name", get(handlers::get_csv_file_handler))
        // CSV 录制 / 模拟 / 主题
        .route("/recording/toggle", post(handlers::toggle_recording_handler))
        .route("/simulation/toggle", post(handlers::toggle_simulation_handler))
        .route("/theme/set", post(handlers::set_theme_handler))
        // 试验信息
        .route("/experiment", get(handlers::get_experiment_handler))
        .route("/experiment", put(handlers::save_experiment_handler))
        // 报表 / 帮助
        .route("/report", post(handlers::generate_report_handler))
        .route("/help", get(handlers::get_help_handler))
        // 中间件执行顺序：从外到内，先权限后认证
        .route_layer(middleware::from_fn_with_state(
            db.clone(),
            fj200c_main_permission_middleware,
        ))
        .route_layer(auth);

    // WebSocket：handler 内部用 ?token= 校验 JWT，不走 Bearer 中间件
    Router::<DatabaseConnection>::new()
        .route("/ws", get(handlers::ws_handler))
        .merge(protected)
}

/// fj200c_main 权限中间件：要求登录且拥有 `Fj200cMainMonitor` 权限
///
/// 委托给通用的 `permission_middleware`，传入 `Permission::Fj200cMainMonitor` 枚举值。
async fn fj200c_main_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::Fj200cMainMonitor, State(db), request, next).await
}
