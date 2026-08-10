//! 公共响应 DTO。
//!
//! fj200c_information / ftj1c 模块共用的载荷类型，供 OpenAPI 文档（utoipa）与前端类型生成使用，
//! 字段与 handlers 层实际返回的 JSON 保持一一对应。
//! 集中定义避免 OpenAPI schema 同名冲突。

use serde::Serialize;

/// 服务运行状态
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ServiceStatus {
    pub running: bool,
}

/// 命令发送结果
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SentResult {
    pub sent: bool,
}

/// 配置保存结果
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SavedResult {
    pub saved: bool,
}

/// 配置文件内容
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConfigContent {
    pub content: String,
}

/// CSV 文件列表
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CsvFileList {
    pub files: Vec<String>,
    pub dir: String,
}

/// CSV 文件内容
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CsvFileContent {
    pub content: String,
}
