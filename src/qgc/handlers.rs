//! # HTTP 端点与 WebSocket 推送（飞控地面站）
//!
//! 端点覆盖服务控制、配置文件、遥测、命令/模式、任务规划与帮助文档：
//!
//! | 功能 | Web 端点 |
//! |---|---|
//! | 服务启停/状态 | `POST /api/qgc/service/start` `POST .../stop` `GET .../status` |
//! | 配置文件 | `GET/PUT /api/qgc/config` |
//! | 遥测快照 | `GET /api/qgc/telemetry` |
//! | 飞控命令 | `POST /api/qgc/command`（arm/disarm/takeoff/land/rtl）|
//! | 模式切换 | `POST /api/qgc/mode` |
//! | 任务规划 | `GET /api/qgc/mission` `PUT ...`（上传）`DELETE ...`（清除）|
//! | 帮助文档 | `GET /api/qgc/help` |
//! | 实时遥测 | `GET /api/qgc/ws?token=`（telemetry / mission_progress / command_ack 事件）|
//!
//! 下行命令经 `state::outbound_sender()` 交给发送线程（统一帧序/目标），
//! 任务上传/清除同理（发送线程负责首页补全与重排）。

use crate::common::dto::{ConfigContent, SavedResult, ServiceStatus};
use crate::common::models::ApiResponse;
use crate::common::ws::{serialize, ws_bridge_with_initial};
use crate::database::DatabaseConnection;
use crate::qgc::models::{
    QgcCommandRequest, QgcMission, QgcModeRequest, QgcMissionUploadRequest, QgcTelemetry,
};
use crate::qgc::state::{self, Outbound};
use crate::qgc::{qgc_tx, QgcEvent};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use std::collections::HashMap;
use std::fs;

/// 飞控命令参数表（MAV_CMD_COMPONENT_ARM_DISARM 等）
///
/// 参数含义按 COMMAND_LONG 规范：params[0..6] 对应 param1..param7。
fn command_params(command: &str, altitude: Option<f32>) -> Option<(u16, [f32; 7])> {
    let mut params = [0.0f32; 7];
    match command {
        "arm" => Some((crate::qgc::mavlink::cmd::COMPONENT_ARM_DISARM, {
            params[0] = 1.0;
            params
        })),
        "disarm" => Some((crate::qgc::mavlink::cmd::COMPONENT_ARM_DISARM, params)),
        "takeoff" => Some((crate::qgc::mavlink::cmd::NAV_TAKEOFF, {
            params[6] = altitude.unwrap_or(10.0).max(1.0);
            params
        })),
        "land" => Some((crate::qgc::mavlink::cmd::NAV_LAND, params)),
        "rtl" => Some((crate::qgc::mavlink::cmd::NAV_RETURN_TO_LAUNCH, params)),
        _ => None,
    }
}

// ---------- 服务控制 ----------

/// 启动飞控通信服务
///
/// # HTTP 端点
/// `POST /api/qgc/service/start`
///
/// # 说明
/// 排他启动（自动停止其他角色后台线程），启动 UDP 接收/发送线程；
/// `[Udp] Mock = true`（默认）时同时启动模拟飞控线程。
#[utoipa::path(
    tag = "qgc",
    post,
    path = "/api/qgc/service/start",
    operation_id = "qgcStartService",
    responses(
        (status = 200, description = "启动成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn start_service_handler(
    State(_db): State<DatabaseConnection>,
) -> Json<ApiResponse<ServiceStatus>> {
    match crate::qgc::service::start_service(qgc_tx()) {
        Ok(()) => Json(ApiResponse::success(ServiceStatus { running: true })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

/// 停止飞控通信服务
///
/// # HTTP 端点
/// `POST /api/qgc/service/stop`
///
/// # 说明
/// 停止操作是异步的，HTTP 请求立即返回，实际停止在后台线程中完成。
#[utoipa::path(
    tag = "qgc",
    post,
    path = "/api/qgc/service/stop",
    operation_id = "qgcStopService",
    responses(
        (status = 200, description = "停止成功", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn stop_service_handler() -> Json<ApiResponse<ServiceStatus>> {
    crate::qgc::service::stop_service();
    Json(ApiResponse::success(ServiceStatus { running: false }))
}

/// 查询飞控服务运行状态
///
/// # HTTP 端点
/// `GET /api/qgc/service/status`
#[utoipa::path(
    tag = "qgc",
    get,
    path = "/api/qgc/service/status",
    operation_id = "qgcGetServiceStatus",
    responses(
        (status = 200, description = "运行状态", body = ApiResponse<ServiceStatus>),
    ),
)]
pub async fn service_status_handler() -> Json<ApiResponse<ServiceStatus>> {
    let running = crate::qgc::service::is_running();
    Json(ApiResponse::success(ServiceStatus { running }))
}

// ---------- 配置文件 ----------

/// 读取配置文件内容
///
/// # HTTP 端点
/// `GET /api/qgc/config`
///
/// # 说明
/// 返回 `config-qgc.ini` 文件的原始内容，供前端配置编辑器使用。
#[utoipa::path(
    tag = "qgc",
    get,
    path = "/api/qgc/config",
    operation_id = "qgcGetConfig",
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
/// # HTTP 端点
/// `PUT /api/qgc/config`
///
/// # 请求体
/// `{ "content": "新的 INI 配置内容" }`
///
/// # 说明
/// 将新配置写入 `config-qgc.ini` 并校验语法；修改的配置在服务重启后生效。
#[utoipa::path(
    tag = "qgc",
    put,
    path = "/api/qgc/config",
    operation_id = "qgcSaveConfig",
    request_body = ConfigContent,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_config_handler(
    Json(req): Json<ConfigContent>,
) -> Json<ApiResponse<SavedResult>> {
    match fs::write(state::CONFIG_PATH, req.content.as_bytes()) {
        Ok(()) => {
            crate::qgc::service::reload_config();
            Json(ApiResponse::success(SavedResult { saved: true }))
        }
        Err(e) => Json(ApiResponse::error(format!("保存配置失败: {}", e))),
    }
}

// ---------- 遥测 / 命令 / 模式 ----------

/// 获取遥测快照
///
/// # HTTP 端点
/// `GET /api/qgc/telemetry`
///
/// # 说明
/// 返回当前遥测快照（由 MAVLink 遥测帧聚合，10Hz 更新）。
/// 实时推送请使用 WebSocket `telemetry` 事件。
#[utoipa::path(
    tag = "qgc",
    get,
    path = "/api/qgc/telemetry",
    operation_id = "qgcGetTelemetry",
    responses(
        (status = 200, description = "遥测快照", body = ApiResponse<QgcTelemetry>),
    ),
)]
pub async fn telemetry_handler() -> Json<ApiResponse<QgcTelemetry>> {
    let t = state::telemetry().read().unwrap_or_else(|e| e.into_inner()).clone();
    Json(ApiResponse::success(t))
}

/// 发送飞控命令
///
/// # HTTP 端点
/// `POST /api/qgc/command`
///
/// # 请求体
/// ```json
/// { "command": "arm", "altitude": 10 }
/// ```
/// 支持命令：`arm` 解锁 / `disarm` 锁定 / `takeoff` 起飞（`altitude` 米）/
/// `land` 降落 / `rtl` 返航。
///
/// # 说明
/// 命令经 COMMAND_LONG 发送，飞控以 COMMAND_ACK 回执
/// （WebSocket `command_ack` 事件，`result_name` 为结果码名称）。
#[utoipa::path(
    tag = "qgc",
    post,
    path = "/api/qgc/command",
    operation_id = "qgcSendCommand",
    request_body = QgcCommandRequest,
    responses(
        (status = 200, description = "命令已发送", body = ApiResponse<bool>),
    ),
)]
pub async fn command_handler(Json(req): Json<QgcCommandRequest>) -> Json<ApiResponse<bool>> {
    let Some((command, params)) = command_params(&req.command, req.altitude) else {
        return Json(ApiResponse::error(format!("不支持的命令: {}", req.command)));
    };
    let Some(tx) = state::outbound_sender() else {
        return Json(ApiResponse::error("服务未启动".to_string()));
    };
    let (_mock, _port, _tip, _tport, sysid, compid, _hb, _hz) = crate::qgc::config::udp_params();
    let frame = crate::qgc::mavlink::encode_command_long(sysid, compid, state::next_seq(), 1, 1, command, params);
    let _ = tx.send(Outbound::Frame(frame));
    Json(ApiResponse::success(true))
}

/// 切换飞行模式
///
/// # HTTP 端点
/// `POST /api/qgc/mode`
///
/// # 请求体
/// ```json
/// { "mode": "guided" }
/// ```
///
/// # 说明
/// 支持 ArduPilot Copter 模式名（stabilize / acro / alt_hold / auto / guided /
/// loiter / rtl / circle / land / drift / sport / poshold / brake / throw），
/// 也接受数字模式 ID（如 `"16"`）。经 SET_MODE 发送（base_mode 带
/// CUSTOM_MODE_ENABLED 位，ArduPilot 协议要求）。
#[utoipa::path(
    tag = "qgc",
    post,
    path = "/api/qgc/mode",
    operation_id = "qgcSetMode",
    request_body = QgcModeRequest,
    responses(
        (status = 200, description = "模式切换指令已发送", body = ApiResponse<bool>),
    ),
)]
pub async fn mode_handler(Json(req): Json<QgcModeRequest>) -> Json<ApiResponse<bool>> {
    let custom_mode = match crate::qgc::mavlink::mode_id(&req.mode) {
        Some(m) => m,
        None => match req.mode.parse::<u32>() {
            Ok(n) => n,
            Err(_) => {
                return Json(ApiResponse::error(format!("未知模式: {}", req.mode)));
            }
        },
    };
    let Some(tx) = state::outbound_sender() else {
        return Json(ApiResponse::error("服务未启动".to_string()));
    };
    let (_mock, _port, _tip, _tport, sysid, compid, _hb, _hz) = crate::qgc::config::udp_params();
    let frame = crate::qgc::mavlink::encode_set_mode(sysid, compid, state::next_seq(), 1, custom_mode);
    let _ = tx.send(Outbound::Frame(frame));
    Json(ApiResponse::success(true))
}

// ---------- 任务规划 ----------

/// 获取任务快照
///
/// # HTTP 端点
/// `GET /api/qgc/mission`
///
/// # 说明
/// 返回任务状态（idle / uploading / downloading / clearing）、航点列表
/// 与飞控当前执行航点序号。下载完成后 `items` 为飞控当前任务
/// （首条 seq=0 为首页，真实飞控要求）。
#[utoipa::path(
    tag = "qgc",
    get,
    path = "/api/qgc/mission",
    operation_id = "qgcGetMission",
    responses(
        (status = 200, description = "任务快照", body = ApiResponse<QgcMission>),
    ),
)]
pub async fn get_mission_handler() -> Json<ApiResponse<QgcMission>> {
    let snapshot = state::mission().lock().unwrap_or_else(|e| e.into_inner()).snapshot();
    Json(ApiResponse::success(snapshot))
}

/// 上传任务（航点规划）
///
/// # HTTP 端点
/// `PUT /api/qgc/mission`
///
/// # 请求体
/// ```json
/// { "items": [ { "seq": 1, "command": 16, "lat": 31.2304, "lon": 121.4737, "altitude": 30 } ] }
/// ```
///
/// # 说明
/// 上传 1~100 个航点（MAV_CMD_NAV_WAYPOINT），序号按数组顺序自动重排；
/// 服务端自动补首页（seq=0，当前位置）后走 MISSION_COUNT → MISSION_ITEM_INT
/// 协议上传，进度经 WebSocket `mission_progress` 事件实时推送，
/// 完成/失败/超时后状态回到 idle。
#[utoipa::path(
    tag = "qgc",
    put,
    path = "/api/qgc/mission",
    operation_id = "qgcUploadMission",
    request_body = QgcMissionUploadRequest,
    responses(
        (status = 200, description = "上传已开始", body = ApiResponse<bool>),
    ),
)]
pub async fn upload_mission_handler(
    Json(req): Json<QgcMissionUploadRequest>,
) -> Json<ApiResponse<bool>> {
    if req.items.is_empty() || req.items.len() > 100 {
        return Json(ApiResponse::error("航点数量须在 1~100 之间".to_string()));
    }
    let Some(tx) = state::outbound_sender() else {
        return Json(ApiResponse::error("服务未启动".to_string()));
    };
    let _ = tx.send(Outbound::MissionUpload(req.items));
    Json(ApiResponse::success(true))
}

/// 从飞控下载任务
///
/// # HTTP 端点
/// `POST /api/qgc/mission/download`
///
/// # 说明
/// 经 MISSION_REQUEST_LIST → MISSION_ITEM_INT 协议从飞控逐条下载当前任务，
/// 下载完成后 `GET /api/qgc/mission` 的 `items` 即为飞控当前任务
/// （首条 seq=0 为首页），进度经 WebSocket `mission_progress` 事件实时推送。
#[utoipa::path(
    tag = "qgc",
    post,
    path = "/api/qgc/mission/download",
    operation_id = "qgcDownloadMission",
    responses(
        (status = 200, description = "下载已开始", body = ApiResponse<bool>),
    ),
)]
pub async fn download_mission_handler() -> Json<ApiResponse<bool>> {
    let Some(tx) = state::outbound_sender() else {
        return Json(ApiResponse::error("服务未启动".to_string()));
    };
    let _ = tx.send(Outbound::MissionDownload);
    Json(ApiResponse::success(true))
}

/// 清除飞控任务
///
/// # HTTP 端点
/// `DELETE /api/qgc/mission`
///
/// # 说明
/// 经 MISSION_CLEAR_ALL 协议清除飞控当前任务，飞控以 MISSION_ACK 回执
/// （WebSocket `mission_progress` 事件，result=ok 即清除成功）。
#[utoipa::path(
    tag = "qgc",
    delete,
    path = "/api/qgc/mission",
    operation_id = "qgcClearMission",
    responses(
        (status = 200, description = "清除已开始", body = ApiResponse<bool>),
    ),
)]
pub async fn clear_mission_handler() -> Json<ApiResponse<bool>> {
    let Some(tx) = state::outbound_sender() else {
        return Json(ApiResponse::error("服务未启动".to_string()));
    };
    let _ = tx.send(Outbound::MissionClear);
    Json(ApiResponse::success(true))
}

// ---------- WebSocket / 帮助 ----------

/// WebSocket 实时遥测
///
/// # HTTP 端点
/// `GET /api/qgc/ws?token=<JWT>`
///
/// # 事件
/// - `telemetry`：遥测快照（10Hz，连接建立时立即推送一次）
/// - `mission_progress`：任务上传/下载/清除进度与结果
/// - `command_ack`：命令回执（command/result/result_name）
///
/// # 鉴权
/// 从查询参数 `token` 提取 JWT 并验证（浏览器 WebSocket 无法设置自定义头）。
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(_db): State<DatabaseConnection>,
) -> Result<Response, axum::http::StatusCode> {
    crate::common::ws::verify_query_token(&params)?;
    Ok(ws.on_upgrade(ws_session))
}

/// WebSocket 会话处理：订阅全局广播通道，连接建立时先推遥测快照
async fn ws_session(socket: WebSocket) {
    let initial = state::telemetry()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let initial_text = serialize(&QgcEvent::Telemetry(initial))
        .ok()
        .map(|s| s.to_string());
    ws_bridge_with_initial(qgc_tx(), socket, "[qgc]", initial_text).await;
}

/// 获取帮助文档
///
/// # HTTP 端点
/// `GET /api/qgc/help`
///
/// # 说明
/// 返回 `help_doc.md` 帮助文档内容（Markdown 文本）。
#[utoipa::path(
    tag = "qgc",
    get,
    path = "/api/qgc/help",
    operation_id = "qgcGetHelp",
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
