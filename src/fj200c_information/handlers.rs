//! # HTTP 端点与 WebSocket 推送
//!
//! 替代原 Tauri 的 5 个 invoke 命令与事件监听：
//!
//! | Tauri 命令/事件 | Web 端点 |
//! |---|---|
//! | `start_service` | `POST /api/fj200c_information/service/start` |
//! | `stop_service` | `POST /api/fj200c_information/service/stop` |
//! | `send_com_data` | `POST /api/fj200c_information/service/command` |
//! | `read_config_file` | `GET /api/fj200c_information/config` |
//! | `save_config_file` | `PUT /api/fj200c_information/config` |
//! | `listen("payload"/"table_data"/"frame")` | `GET /api/fj200c_information/ws`（type 字段区分）|
//!
//! ## WebSocket 鉴权
//!
//! 浏览器 WebSocket API 无法设置自定义头，因此令牌通过查询参数传递：
//! `ws://host:port/api/fj200c_information/ws?token=<JWT>`。握手时用 `jwt::verify_token`
//! 校验，无效则拒绝升级（401）。
//!
//! ## Axum 提取器说明
//!
//! - `State(_db)`：从路由状态中提取数据库连接池（由 `fj200c_information_router` 注入）
//! - `Json(req)`：反序列化 JSON 请求体为结构体
//! - `Query(params)`：从 URL 查询参数中提取 `HashMap<String, String>`
//! - `WebSocketUpgrade`：自动处理 HTTP → WebSocket 协议升级

use crate::common::dto::{
    ConfigContent, CsvFileContent, CsvFileList, SavedResult, SentResult, ServiceStatus,
};
use crate::common::jwt;
use crate::common::models::ApiResponse;
use crate::database::DatabaseConnection;
use crate::fj200c_information::service;
use crate::fj200c_information::session::frame_bundle;
use crate::fj200c_information::state::CONFIG_PATH;
use crate::fj200c_information::{decode, fj200c_information_tx, Fj200cInformationEvent, TableRow};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// 发送命令的请求体结构（JSON 反序列化用）
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SendCommandRequest {
    /// 十六进制命令字符串，如 "EB 90 64 EF 00"
    pub hex: String,
}

/// 保存配置文件的请求体结构
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SaveConfigRequest {
    /// 配置文件的完整内容（INI 格式）
    pub content: String,
}

// ---------- 服务控制 ----------

/// 启动发动机监控服务
///
/// 读取配置文件，为每个启用的连接启动会话线程。
/// 已运行时返回错误提示，防止重复启动。
#[utoipa::path(
    tag = "fj200c_information",
    post,
    path = "/api/fj200c_information/service/start",
    operation_id = "fj200c_informationStartService",
    responses(
        (status = 200, description = "启动成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn start_service_handler(
    State(_db): State<DatabaseConnection>,
) -> Json<ApiResponse<ServiceStatus>> {
    match service::start_service(fj200c_information_tx()) {
        Ok(()) => Json(ApiResponse::success(ServiceStatus { running: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// 停止发动机监控服务
///
/// 设置停止信号后在独立线程中等待所有会话线程退出，避免阻塞 HTTP 请求。
#[utoipa::path(
    tag = "fj200c_information",
    post,
    path = "/api/fj200c_information/service/stop",
    operation_id = "fj200c_informationStopService",
    responses(
        (status = 200, description = "停止成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn stop_service_handler() -> Json<ApiResponse<ServiceStatus>> {
    service::stop_service();
    Json(ApiResponse::success(ServiceStatus { running: false }))
}

/// 查询发动机监控服务运行状态
#[utoipa::path(
    tag = "fj200c_information",
    get,
    path = "/api/fj200c_information/service/status",
    operation_id = "fj200c_informationGetServiceStatus",
    responses(
        (status = 200, description = "运行状态", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn service_status_handler() -> Json<ApiResponse<ServiceStatus>> {
    let running = service::is_running();
    Json(ApiResponse::success(ServiceStatus { running }))
}

/// 向所有会话线程发送十六进制命令
///
/// 命令通过 `mpsc` 通道广播到所有会话线程，由各线程写入对应的 IO 设备。
#[utoipa::path(
    tag = "fj200c_information",
    post,
    path = "/api/fj200c_information/service/command",
    operation_id = "fj200c_informationSendCommand",
    request_body = SendCommandRequest,
    responses(
        (status = 200, description = "发送成功", body = ApiResponse<SentResult>),
    ),
)]
pub async fn send_command_handler(
    Json(req): Json<SendCommandRequest>,
) -> Json<ApiResponse<SentResult>> {
    match service::send_command(&req.hex) {
        Ok(()) => Json(ApiResponse::success(SentResult { sent: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

// ---------- 配置文件 ----------

/// 读取 config-fj200c_information.ini 配置文件的原始内容
///
/// 返回 INI 格式的纯文本，前端可直接展示和编辑。
#[utoipa::path(
    tag = "fj200c_information",
    get,
    path = "/api/fj200c_information/config",
    operation_id = "fj200c_informationGetConfig",
    responses(
        (status = 200, description = "配置文件内容", body = ApiResponse<ConfigContent>),
    ),
)]
pub async fn read_config_handler() -> Json<ApiResponse<ConfigContent>> {
    match fs::read_to_string(CONFIG_PATH) {
        Ok(content) => Json(ApiResponse::success(ConfigContent { content })),
        Err(e) => Json(ApiResponse::error(format!("读取配置失败: {}", e))),
    }
}

/// 保存配置文件内容到 config-fj200c_information.ini
///
/// 覆盖写入整个文件内容，前端提交前应提示用户确认。
#[utoipa::path(
    tag = "fj200c_information",
    put,
    path = "/api/fj200c_information/config",
    operation_id = "fj200c_informationSaveConfig",
    request_body = SaveConfigRequest,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_config_handler(
    Json(req): Json<SaveConfigRequest>,
) -> Json<ApiResponse<SavedResult>> {
    match fs::write(CONFIG_PATH, req.content.as_bytes()) {
        Ok(()) => Json(ApiResponse::success(SavedResult { saved: true })),
        Err(e) => Json(ApiResponse::error(format!("保存配置失败: {}", e))),
    }
}

// ---------- CSV 数据记录浏览 ----------

/// 获取 CSV 文件存储目录（从配置或默认值）
fn csv_dir() -> String {
    crate::fj200c_information::config::global()
        .map(|c| c.get_or("CSV", "Dir", "csv"))
        .unwrap_or_else(|| "csv".to_string())
}

/// 列出 CSV 目录下所有 CSV 文件（按文件名倒序，新文件在前）
#[utoipa::path(
    tag = "fj200c_information",
    get,
    path = "/api/fj200c_information/csv/files",
    operation_id = "fj200c_informationListCsvFiles",
    responses(
        (status = 200, description = "CSV 文件列表", body = ApiResponse<CsvFileList>),
    ),
)]
pub async fn list_csv_files_handler() -> Json<ApiResponse<CsvFileList>> {
    let dir = csv_dir();
    let files: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("csv"))
                    .unwrap_or(false)
            })
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => Vec::new(),
    };
    let mut files = files;
    files.sort_by(|a, b| b.cmp(a)); // 新的在前
    Json(ApiResponse::success(CsvFileList { files, dir }))
}

/// 读取指定 CSV 文件的内容
///
/// 仅允许文件名（禁止目录穿越），防止路径遍历攻击。
#[utoipa::path(
    tag = "fj200c_information",
    get,
    path = "/api/fj200c_information/csv/{name}",
    operation_id = "fj200c_informationGetCsvFile",
    params(
        ("name" = String, Path, description = "CSV 文件名"),
    ),
    responses(
        (status = 200, description = "文件内容", body = ApiResponse<CsvFileContent>),
    ),
)]
pub async fn get_csv_file_handler(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse<CsvFileContent>> {
    // 防目录穿越：仅允许文件名
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Json(ApiResponse::error("非法的文件名".to_string()));
    }
    let dir = csv_dir();
    let path = std::path::Path::new(&dir).join(&name);
    match fs::read_to_string(&path) {
        Ok(content) => Json(ApiResponse::success(CsvFileContent { content })),
        Err(e) => Json(ApiResponse::error(format!("读取文件失败: {}", e))),
    }
}

// ---------- WebSocket ----------

/// WebSocket 协议升级处理器
///
/// 从查询参数中提取 JWT 令牌进行鉴权，通过后执行 HTTP → WebSocket 协议升级。
/// 浏览器 WebSocket API 不支持自定义头，因此令牌通过 `?token=` 传递。
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(_db): State<DatabaseConnection>,
) -> Result<Response, axum::http::StatusCode> {
    let token = params
        .get("token")
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    jwt::verify_token(token).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    Ok(ws.on_upgrade(|socket| ws_session(socket)))
}

/// WebSocket 会话主循环
///
/// 连接建立时立即发送一次当前表格快照，确保新客户端能看到最新数据，
/// 之后通过公共 `ws_bridge_with_initial` 转发广播事件。
async fn ws_session(socket: WebSocket) {
    let snapshot = build_table_snapshot();
    let initial = if snapshot.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&Fj200cInformationEvent::TableData {
                connection_index: 0,
                rows: snapshot,
            })
            .unwrap(),
        )
    };
    crate::common::ws::ws_bridge_with_initial(fj200c_information_tx(), socket, "[fj200c_information]", initial).await;
}

/// 从全局帧缓存构建当前表格快照（连接时发送）
fn build_table_snapshot() -> Vec<TableRow> {
    let fields = frame_bundle().frames();
    if fields.is_empty() {
        return Vec::new();
    }
    decode::CSV_HEADERS
        .iter()
        .zip(fields.iter())
        .map(|(field, value)| TableRow {
            field: field.to_string(),
            value: value.clone(),
        })
        .collect()
}
