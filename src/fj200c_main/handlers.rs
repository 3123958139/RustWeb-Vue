//! # HTTP 端点与 WebSocket 推送（fj200c_main 角色域）
//!
//! 替代原 Tauri fj200c_information 桌面应用的 11 个 invoke 命令与事件监听，
//! 提供 15 个 HTTP 端点 + 1 个 WebSocket 连接：
//!
//! | Tauri 命令/事件 | Web 端点 |
//! |---|---|
//! | `start_service` | `POST /api/fj200c_main/service/start` |
//! | `stop_service` | `POST /api/fj200c_main/service/stop` |
//! | `get_service_status` | `GET /api/fj200c_main/service/status` |
//! | `send_ecu_data` | `POST /api/fj200c_main/service/command` |
//! | `read_config_file` | `GET /api/fj200c_main/config` |
//! | `save_config_file` | `PUT /api/fj200c_main/config` |
//! | `list_csv_files` | `GET /api/fj200c_main/csv/files` |
//! | `read_csv_file` | `GET /api/fj200c_main/csv/{name}` |
//! | `toggle_csv_recording` | `POST /api/fj200c_main/recording/toggle` |
//! | `toggle_simulation` | `POST /api/fj200c_main/simulation/toggle` |
//! | `set_theme` | `POST /api/fj200c_main/theme/set` |
//! | `get_experiment_info` | `GET /api/fj200c_main/experiment` |
//! | `save_experiment_info` | `PUT /api/fj200c_main/experiment` |
//! | `generate_report` | `POST /api/fj200c_main/report` |
//! | `read_readme_file` | `GET /api/fj200c_main/help` |
//! | `listen("port_data"/...)` | `GET /api/fj200c_main/ws`（type 字段区分事件）|
//!
//! ## WebSocket 鉴权
//!
//! 浏览器 WebSocket API 无法设置自定义头，因此令牌通过查询参数传递：
//! `ws://host:port/api/fj200c_main/ws?token=<JWT>`。握手时用 `jwt::verify_token`
//! 校验，无效则拒绝升级（401）。
//!
//! ## WebSocket 初始快照
//!
//! 连接建立时先发送一次三端口（ECU/ADAM/DYNO）解码快照（JSON 数组），
//! 模拟 Tauri `get_port_snapshot`，确保新客户端能看到最新数据。

use crate::common::dto::{
    ConfigContent, CsvFileContent, CsvFileList, SavedResult, SentResult, ServiceStatus,
};
use crate::common::jwt;
use crate::common::models::ApiResponse;
use crate::database::DatabaseConnection;
use crate::fj200c_main::fj200c_main_tx;
use crate::fj200c_main::service;
use crate::fj200c_main::state;
use crate::fj200c_main::types::{ChannelData, ExperimentInfo, ReportOutput};
use crate::fj200c_main::Fj200cMainEvent;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

// ============ 请求/响应 DTO ============

/// 发送 ECU 指令的请求体
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SendCommandRequest {
    /// 十六进制命令字符串，如 "EB901000000000000000000000000000"
    pub hex: String,
}

/// 保存配置文件的请求体
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SaveConfigRequest {
    /// INI 配置文件的完整内容
    pub content: String,
}

/// 生成报表的请求体
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReportRequest {
    /// CSV 文件名（用于推导试验信息文件名）
    pub file_name: String,
    /// CSV 文件内容
    pub content: String,
    /// 状态点（逗号分隔的 RPM 值）
    pub state_points: String,
}

/// 主题切换请求体
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThemeRequest {
    /// 是否深色主题
    pub is_dark: bool,
}

/// CSV 录制状态响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecordingState {
    /// 是否正在录制
    pub recording: bool,
}

/// 模拟运行状态响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SimulationState {
    /// 是否正在模拟
    pub simulating: bool,
}

/// 主题状态响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThemeState {
    /// 是否深色主题
    pub is_dark: bool,
}

// ============ 服务控制 ============

/// 启动 fj200c_main 测控服务
///
/// 加载配置 → 初始化串口 → 启动解码/发送/CSV 线程。
/// 已运行时返回错误。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/service/start",
    operation_id = "fj200cMainStartService",
    responses(
        (status = 200, description = "启动成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn start_service_handler(
    State(_db): State<DatabaseConnection>,
) -> Json<ApiResponse<ServiceStatus>> {
    match service::start_service(fj200c_main_tx()) {
        Ok(()) => Json(ApiResponse::success(ServiceStatus { running: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// 停止 fj200c_main 测控服务
///
/// 设置停止信号后清理串口与线程，立即返回。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/service/stop",
    operation_id = "fj200cMainStopService",
    responses(
        (status = 200, description = "停止成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn stop_service_handler() -> Json<ApiResponse<ServiceStatus>> {
    service::stop_service();
    Json(ApiResponse::success(ServiceStatus { running: false }))
}

/// 查询服务运行状态
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/service/status",
    operation_id = "fj200cMainGetServiceStatus",
    responses(
        (status = 200, description = "运行状态", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn service_status_handler() -> Json<ApiResponse<ServiceStatus>> {
    let running = service::is_running();
    Json(ApiResponse::success(ServiceStatus { running }))
}

/// 发送 ECU 指令（十六进制帧）
///
/// 帧字节存入全局 `ecu_send_data`，由 100ms 周期发送线程写入串口。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/service/command",
    operation_id = "fj200cMainSendCommand",
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

// ============ 配置文件 ============

/// 读取 config-fj200c_main.ini 配置文件内容
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/config",
    operation_id = "fj200cMainGetConfig",
    responses(
        (status = 200, description = "配置文件内容", body = ApiResponse<ConfigContent>),
    ),
)]
pub async fn read_config_handler() -> Json<ApiResponse<ConfigContent>> {
    match fs::read_to_string(state::CONFIG_PATH) {
        Ok(content) => Json(ApiResponse::success(ConfigContent { content })),
        Err(e) => Json(ApiResponse::error(format!("读取配置失败: {}", e))),
    }
}

/// 保存配置文件内容
///
/// 覆盖写入 config-fj200c_main.ini，修改在服务重启后生效。
#[utoipa::path(
    tag = "fj200c_main",
    put,
    path = "/api/fj200c_main/config",
    operation_id = "fj200cMainSaveConfig",
    request_body = SaveConfigRequest,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_config_handler(
    Json(req): Json<SaveConfigRequest>,
) -> Json<ApiResponse<SavedResult>> {
    match fs::write(state::CONFIG_PATH, req.content.as_bytes()) {
        Ok(()) => Json(ApiResponse::success(SavedResult { saved: true })),
        Err(e) => Json(ApiResponse::error(format!("保存配置失败: {}", e))),
    }
}

// ============ CSV 数据浏览 ============

/// 获取 CSV 目录（从配置 `[CSV] Dir` 读取，默认 `csv`）
fn csv_dir() -> String {
    crate::fj200c_main::config::global()
        .map(|c| c.get_or("CSV", "Dir", "csv"))
        .unwrap_or_else(|| "csv".to_string())
}

/// 列出 CSV 目录下所有 CSV 文件（按文件名倒序，新文件在前）
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/csv/files",
    operation_id = "fj200cMainListCsvFiles",
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
    files.sort_by(|a, b| b.cmp(a));
    Json(ApiResponse::success(CsvFileList { files, dir }))
}

/// 读取指定 CSV 文件的内容
///
/// 仅允许文件名（禁止目录穿越），防止路径遍历攻击。
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/csv/{name}",
    operation_id = "fj200cMainGetCsvFile",
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

// ============ CSV 录制切换 ============

/// 切换 CSV 数据录制状态（开始/停止）
///
/// 状态机 0→1→2→0：
/// - 0→1：创建试验信息文件 + 录制文件，写表头
/// - 1→2：进入持续写入
/// - 2→0：停止录制，刷新文件
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/recording/toggle",
    operation_id = "fj200cMainToggleRecording",
    responses(
        (status = 200, description = "录制状态", body = ApiResponse<RecordingState>),
    ),
)]
pub async fn toggle_recording_handler() -> Json<ApiResponse<RecordingState>> {
    let tx = fj200c_main_tx();
    service::toggle_csv_recording(&tx);
    let recording = state::CSV_RECORDING.load(std::sync::atomic::Ordering::Relaxed) != 0;
    Json(ApiResponse::success(RecordingState { recording }))
}

// ============ 模拟运行切换 ============

/// 切换模拟运行状态（启动/停止）
///
/// 启动时通过进程内直通驱动三路解码与推送（无需虚拟串口）；
/// 停止时设置停止标志，模拟线程退出。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/simulation/toggle",
    operation_id = "fj200cMainToggleSimulation",
    responses(
        (status = 200, description = "模拟状态", body = ApiResponse<SimulationState>),
    ),
)]
pub async fn toggle_simulation_handler() -> Json<ApiResponse<SimulationState>> {
    let tx = fj200c_main_tx();
    service::toggle_simulation(&tx);
    let simulating = state::SIMULATION_MODE.load(std::sync::atomic::Ordering::Relaxed);
    Json(ApiResponse::success(SimulationState { simulating }))
}

// ============ 主题切换 ============

/// 设置主题（深色/浅色）
///
/// 服务端存储主题状态并广播给所有 WS 客户端，前端按钮文案本地响应。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/theme/set",
    operation_id = "fj200cMainSetTheme",
    request_body = ThemeRequest,
    responses(
        (status = 200, description = "主题状态", body = ApiResponse<ThemeState>),
    ),
)]
pub async fn set_theme_handler(
    Json(req): Json<ThemeRequest>,
) -> Json<ApiResponse<ThemeState>> {
    let tx = fj200c_main_tx();
    service::set_theme(req.is_dark, &tx);
    Json(ApiResponse::success(ThemeState { is_dark: req.is_dark }))
}

// ============ 试验信息 ============

/// 获取试验信息（从 GlobalVar 读取）
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/experiment",
    operation_id = "fj200cMainGetExperiment",
    responses(
        (status = 200, description = "试验信息", body = ApiResponse<ExperimentInfo>),
    ),
)]
pub async fn get_experiment_handler() -> Json<ApiResponse<ExperimentInfo>> {
    Json(ApiResponse::success(service::get_experiment_info()))
}

/// 保存试验信息（写入 GlobalVar）
#[utoipa::path(
    tag = "fj200c_main",
    put,
    path = "/api/fj200c_main/experiment",
    operation_id = "fj200cMainSaveExperiment",
    request_body = ExperimentInfo,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_experiment_handler(
    Json(req): Json<ExperimentInfo>,
) -> Json<ApiResponse<SavedResult>> {
    match service::save_experiment_info(&req) {
        Ok(()) => Json(ApiResponse::success(SavedResult { saved: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

// ============ 报表生成 ============

/// 生成试验报表
///
/// 解析 CSV → 状态点滤波 → 均值 → 标况换算 → 填充 → 最小二乘插值，
/// 返回 4 张表格数据（基本信息/性能/标准/设计点）。
///
/// 无需服务启动，选中 CSV 即可生成；试验信息缺省时使用空值/默认值。
#[utoipa::path(
    tag = "fj200c_main",
    post,
    path = "/api/fj200c_main/report",
    operation_id = "fj200cMainGenerateReport",
    request_body = GenerateReportRequest,
    responses(
        (status = 200, description = "报表数据", body = ApiResponse<ReportOutput>),
    ),
)]
pub async fn generate_report_handler(
    Json(req): Json<GenerateReportRequest>,
) -> Json<ApiResponse<ReportOutput>> {
    // 无需服务启动即可生成：process_report_csv 内部对 GlobalVar/试验信息缺省均能容错
    match crate::fj200c_main::report::process_report_csv(
        &req.file_name,
        &req.content,
        &req.state_points,
    ) {
        Ok(output) => Json(ApiResponse::success(output)),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

// ============ 帮助文档 ============

/// 获取用户操作说明（help_doc.md，编译期内嵌）
#[utoipa::path(
    tag = "fj200c_main",
    get,
    path = "/api/fj200c_main/help",
    operation_id = "fj200cMainGetHelp",
    responses(
        (status = 200, description = "帮助文档内容", body = ApiResponse<ConfigContent>),
    ),
)]
pub async fn get_help_handler() -> Json<ApiResponse<ConfigContent>> {
    const HELP_TEXT: &str = include_str!("help_doc.md");
    Json(ApiResponse::success(ConfigContent {
        content: HELP_TEXT.to_string(),
    }))
}

// ============ WebSocket ============

/// WebSocket 升级处理器
///
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
    let token = params
        .get("token")
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    jwt::verify_token(token).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    Ok(ws.on_upgrade(|socket| ws_session(socket)))
}

/// WebSocket 会话主循环
///
/// 连接建立时先发送一次三端口解码快照（JSON 数组），确保新客户端能看到最新数据，
/// 之后通过公共 `ws_bridge_with_initial` 转发广播事件。
async fn ws_session(socket: WebSocket) {
    let initial = build_initial_snapshot();
    crate::common::ws::ws_bridge_with_initial(
        fj200c_main_tx(),
        socket,
        "[fj200c_main]",
        initial,
    )
    .await;
}

/// 构建连接建立时的初始快照（ECU/ADAM/DYNO 三端口当前解码值）
///
/// 返回一个包含 3 个 `PortData` 事件的 JSON 数组字符串。
/// 若服务未启动（SharedPortData 未初始化），返回 `None`。
fn build_initial_snapshot() -> Option<String> {
    let shared = state::shared_port_data()?;

    let ecu = shared.ecu_decoded.load();
    let adam = shared.adam_decoded.load();
    let dyno = shared.dyno_decoded.load();

    let events = vec![
        Fj200cMainEvent::PortData {
            connection_index: 0,
            hex: hex_string(&shared.ecu_raw.read()),
            fields: std::sync::Arc::new(ChannelData::Ecu((**ecu).clone())),
        },
        Fj200cMainEvent::PortData {
            connection_index: 1,
            hex: hex_string(&shared.adam_raw.read()),
            fields: std::sync::Arc::new(ChannelData::Adam((**adam).clone())),
        },
        Fj200cMainEvent::PortData {
            connection_index: 2,
            hex: hex_string(&shared.dyno_raw.read()),
            fields: std::sync::Arc::new(ChannelData::Dyno((**dyno).clone())),
        },
    ];

    serde_json::to_string(&events).ok()
}

/// 将字节数组转为连续大写十六进制字符串（查表实现）
fn hex_string(bytes: &[u8]) -> String {
    crate::common::utils::format_hex_compact(bytes)
}
