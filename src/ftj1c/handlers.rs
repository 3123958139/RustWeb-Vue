//! # HTTP 端点与 WebSocket 推送
//!
//! 替代原 demo-test3-ftj 的 Tauri invoke 命令与事件监听：
//!
//! | Tauri 命令/事件 | Web 端点 |
//! |---|---|
//! | `start_service` | `POST /api/ftj1c/service/start` |
//! | `stop_service` | `POST /api/ftj1c/service/stop` |
//! | `get_ip_config` | `GET /api/ftj1c/ip-config` |
//! | `read_config_file` | `GET /api/ftj1c/config` |
//! | `save_config_file` | `PUT /api/ftj1c/config` |
//! | `listen("udp-data")` | `GET /api/ftj1c/ws`（type=udp_data 事件）|
//!
//! ## WebSocket 鉴权
//!
//! 浏览器 WebSocket API 无法设置自定义头，因此令牌通过查询参数传递：
//! `ws://host:port/api/ftj1c/ws?token=<JWT>`。握手时用 `jwt::verify_token`
//! 校验，无效则拒绝升级（401）。
//!
//! ## Axum 提取器说明
//! - `State(_db)`: 从应用状态提取数据库连接（`DatabaseConnection`）
//! - `Json(req)`: 从请求体提取 JSON 并反序列化为 `SaveConfigRequest`
//! - `Query(params)`: 从 URL 查询参数提取 `HashMap<String, String>`
//! - `WebSocketUpgrade`: 处理 WebSocket 升级请求

use crate::common::dto::{ConfigContent, SavedResult, ServiceStatus};
use crate::common::models::ApiResponse;
use crate::database::DatabaseConnection;
use crate::ftj1c::ftj1c_tx;
use crate::ftj1c::models::{Ftj1cSaveConfigRequest, IpConfig};
use crate::ftj1c::service;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use std::collections::HashMap;
use std::fs;

// ---------- 服务控制 ----------

/// 启动通信监控服务
///
/// # HTTP 端点
/// `POST /api/ftj1c/service/start`
///
/// # 响应
/// - 成功: `{ "code": 200, "data": { "running": true } }`
/// - 失败: `{ "code": 500, "message": "错误原因" }`
#[utoipa::path(
    tag = "ftj1c",
    post,
    path = "/api/ftj1c/service/start",
    operation_id = "ftj1cStartService",
    responses(
        (status = 200, description = "启动成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn start_service_handler(
    State(_db): State<DatabaseConnection>,
) -> Json<ApiResponse<ServiceStatus>> {
    match service::start_service(ftj1c_tx()) {
        Ok(()) => Json(ApiResponse::success(ServiceStatus { running: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// 停止通信监控服务
///
/// # HTTP 端点
/// `POST /api/ftj1c/service/stop`
///
/// # 说明
/// 停止操作是异步的，HTTP 请求会立即返回，实际停止在后台线程中完成。
#[utoipa::path(
    tag = "ftj1c",
    post,
    path = "/api/ftj1c/service/stop",
    operation_id = "ftj1cStopService",
    responses(
        (status = 200, description = "停止成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn stop_service_handler() -> Json<ApiResponse<ServiceStatus>> {
    service::stop_service();
    Json(ApiResponse::success(ServiceStatus { running: false }))
}

/// 查询服务运行状态
///
/// # HTTP 端点
/// `GET /api/ftj1c/service/status`
///
/// # 响应
/// `{ "code": 200, "data": { "running": true/false } }`
#[utoipa::path(
    tag = "ftj1c",
    get,
    path = "/api/ftj1c/service/status",
    operation_id = "ftj1cGetServiceStatus",
    responses(
        (status = 200, description = "运行状态", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn service_status_handler() -> Json<ApiResponse<ServiceStatus>> {
    let running = service::is_running();
    Json(ApiResponse::success(ServiceStatus { running }))
}

// ---------- IP 配置 ----------

/// 获取 IP 配置
///
/// # HTTP 端点
/// `GET /api/ftj1c/ip-config`
///
/// # 响应
/// 返回 16 组 IP 和端口配置（ip1, port1, ..., ip16, port16）。
#[utoipa::path(
    tag = "ftj1c",
    get,
    path = "/api/ftj1c/ip-config",
    operation_id = "ftj1cGetIpConfig",
    responses(
        (status = 200, description = "IP 配置", body = ApiResponse<IpConfig>),
    ),
)]
pub async fn ip_config_handler() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(service::get_ip_config()))
}

// ---------- 配置文件 ----------

/// 读取配置文件内容
///
/// # HTTP 端点
/// `GET /api/ftj1c/config`
///
/// # 响应
/// `{ "code": 200, "data": { "content": "[Udp]\nMock = true\n..." } }`
///
/// # 说明
/// 返回 `config-ftj1c.ini` 文件的原始内容，供前端配置编辑器使用。
#[utoipa::path(
    tag = "ftj1c",
    get,
    path = "/api/ftj1c/config",
    operation_id = "ftj1cGetConfig",
    responses(
        (status = 200, description = "配置文件内容", body = ApiResponse<ConfigContent>),
    ),
)]
pub async fn read_config_handler() -> Json<ApiResponse<ConfigContent>> {
    match fs::read_to_string(crate::ftj1c::state::CONFIG_PATH) {
        Ok(content) => Json(ApiResponse::success(ConfigContent { content })),
        Err(e) => Json(ApiResponse::error(format!("读取配置失败: {}", e))),
    }
}

/// 保存配置文件内容
///
/// # HTTP 端点
/// `PUT /api/ftj1c/config`
///
/// # 请求体
/// `{ "content": "新的 INI 配置内容" }`
///
/// # 说明
/// 将新配置写入 `config-ftj1c.ini` 文件，并重新加载到全局配置实例。
/// 修改的配置在服务重启后才会生效。
#[utoipa::path(
    tag = "ftj1c",
    put,
    path = "/api/ftj1c/config",
    operation_id = "ftj1cSaveConfig",
    request_body = Ftj1cSaveConfigRequest,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_config_handler(
    Json(req): Json<Ftj1cSaveConfigRequest>,
) -> Json<ApiResponse<SavedResult>> {
    match fs::write(crate::ftj1c::state::CONFIG_PATH, req.content.as_bytes()) {
        Ok(()) => {
            service::reload_config();
            Json(ApiResponse::success(SavedResult { saved: true }))
        }
        Err(e) => Json(ApiResponse::error(format!("保存配置失败: {}", e))),
    }
}

// ---------- WebSocket ----------

/// WebSocket 升级处理器
///
/// # HTTP 端点
/// `GET /api/ftj1c/ws?token=<JWT>`
///
/// # 鉴权
/// 从查询参数 `token` 提取 JWT 并验证。浏览器 WebSocket API 无法设置自定义头，
/// 因此使用查询参数传递令牌。
///
/// # 错误码
/// - `401 Unauthorized`: 令牌缺失或无效
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(_db): State<DatabaseConnection>,
) -> Result<Response, axum::http::StatusCode> {
    crate::common::ws::verify_query_token(&params)?;

    Ok(ws.on_upgrade(|socket| ws_session(socket)))
}

/// WebSocket 会话处理
///
/// 通过公共 `ws_bridge` 订阅全局广播通道 `ftj1c_tx()`，
/// 将 UDP 数据事件序列化为 JSON 推送给客户端。
async fn ws_session(socket: WebSocket) {
    crate::common::ws::ws_bridge(ftj1c_tx(), socket, "[ftj1c]").await;
}

#[utoipa::path(
tag = "ftj1c",
get,
path = "/api/ftj1c/help",
operation_id = "ftj1cGetHelp",
responses(
(status=200,description="帮助文档内容", body = ApiResponse<ConfigContent>),
)
)]
pub async fn get_help_handler() -> Json<ApiResponse<ConfigContent>> {
    const HELP_TEXT: &str = include_str!("help_doc.md");
    Json(ApiResponse::success(ConfigContent {
        content: HELP_TEXT.to_string(),
    }))
}
