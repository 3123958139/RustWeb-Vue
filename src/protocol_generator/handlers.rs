//! protocol_generator 角色模块 handler：通信协议生成 API。
//!
//! 端点全部要求 `ProtocolGeneratorMonitor` 权限（见 `routes.rs`）。

use crate::common::dto::SavedResult;
use crate::common::error::AppError;
use crate::common::models::ApiResponse;
use crate::protocol_generator::models::{CsvParseRequest, ParameterDef, ProtocolExportRequest, TextContent};
use crate::protocol_generator::services::ProtocolGeneratorService;
use axum::{
    http::header,
    response::{IntoResponse, Response},
    Json,
};

/// 读取默认参数表（首次访问自动写入种子内容并返回）
#[utoipa::path(
    tag = "protocol_generator",
    get,
    path = "/api/protocol_generator/default-csv",
    operation_id = "protocolGeneratorGetDefaultCsv",
    responses(
        (status = 200, description = "默认参数表列表", body = ApiResponse<Vec<ParameterDef>>),
    ),
)]
pub async fn get_default_csv() -> Result<Json<ApiResponse<Vec<ParameterDef>>>, AppError> {
    let data = ProtocolGeneratorService::load_default_csv()
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(data)))
}

/// 保存默认参数表（写入服务器运行目录 parameters.csv，UTF-8 BOM）
#[utoipa::path(
    tag = "protocol_generator",
    put,
    path = "/api/protocol_generator/default-csv",
    operation_id = "protocolGeneratorSaveDefaultCsv",
    request_body = Vec<ParameterDef>,
    responses(
        (status = 200, description = "保存结果", body = ApiResponse<SavedResult>),
    ),
)]
pub async fn save_default_csv(
    Json(data): Json<Vec<ParameterDef>>,
) -> Result<Json<ApiResponse<SavedResult>>, AppError> {
    ProtocolGeneratorService::save_default_csv(&data)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(SavedResult { saved: true })))
}

/// 导出协议表 Markdown 文本
#[utoipa::path(
    tag = "protocol_generator",
    post,
    path = "/api/protocol_generator/markdown",
    operation_id = "protocolGeneratorExportMarkdown",
    request_body = ProtocolExportRequest,
    responses(
        (status = 200, description = "Markdown 文本", body = ApiResponse<TextContent>),
    ),
)]
pub async fn export_markdown(
    Json(req): Json<ProtocolExportRequest>,
) -> Result<Json<ApiResponse<TextContent>>, AppError> {
    let content = ProtocolGeneratorService::export_markdown(&req.title, &req.data)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(TextContent { content })))
}

/// 导出协议表 Excel 二进制（xlsx，前端 Blob 下载）
#[utoipa::path(
    tag = "protocol_generator",
    post,
    path = "/api/protocol_generator/excel",
    operation_id = "protocolGeneratorExportExcel",
    request_body = ProtocolExportRequest,
    responses(
        (status = 200, description = "xlsx 文件二进制流", content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ),
)]
pub async fn export_excel(
    Json(req): Json<ProtocolExportRequest>,
) -> Result<Response, AppError> {
    let bytes = ProtocolGeneratorService::export_excel(&req.title, &req.data)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"protocol.xlsx\""),
        ],
        bytes,
    )
        .into_response())
}

/// 解析 CSV 文本（前端上传文件内容，兼容 UTF-8 BOM）
#[utoipa::path(
    tag = "protocol_generator",
    post,
    path = "/api/protocol_generator/csv/parse",
    operation_id = "protocolGeneratorParseCsv",
    request_body = CsvParseRequest,
    responses(
        (status = 200, description = "解析后的参数表", body = ApiResponse<Vec<ParameterDef>>),
    ),
)]
pub async fn parse_csv(
    Json(req): Json<CsvParseRequest>,
) -> Result<Json<ApiResponse<Vec<ParameterDef>>>, AppError> {
    let data = ProtocolGeneratorService::parse_csv(&req.content)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(data)))
}

/// 参数表序列化为 CSV 文本（UTF-8 BOM，前端下载）
#[utoipa::path(
    tag = "protocol_generator",
    post,
    path = "/api/protocol_generator/csv/serialize",
    operation_id = "protocolGeneratorSerializeCsv",
    request_body = Vec<ParameterDef>,
    responses(
        (status = 200, description = "CSV 文本", body = ApiResponse<TextContent>),
    ),
)]
pub async fn serialize_csv(
    Json(data): Json<Vec<ParameterDef>>,
) -> Result<Json<ApiResponse<TextContent>>, AppError> {
    let content = ProtocolGeneratorService::serialize_csv(&data)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(TextContent { content })))
}