//! 二级子模块：协议生成器业务模型（DTO 集中定义，供 OpenAPI / orval 生成前端类型）。

use serde::{Deserialize, Serialize};

/// 协议字段（通信协议表一行）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProtocolField {
    /// 序号（按数据类型大小自动重排）
    pub index: u32,
    /// 字节范围（如 "0-3" / "4~N"）
    #[serde(rename = "byteRange")]
    pub byte_range: String,
    /// 参数名称
    pub name: String,
    /// 单位
    pub unit: String,
    /// 数据类型（C# 内置类型）
    #[serde(rename = "dataType")]
    pub data_type: String,
    /// 长度（仅可变类型 string/byte[] 使用）
    #[serde(rename = "length", default)]
    pub length: Option<u32>,
    /// 备注
    pub remark: String,
}

/// 参数表条目（CSV 参数表一行）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ParameterDef {
    /// 参数名称
    pub name: String,
    /// 别名
    pub alias: String,
    /// 单位
    pub unit: String,
    /// 数据类型
    #[serde(rename = "dataType")]
    pub data_type: String,
    /// 备注
    pub remark: String,
}

/// 协议导出请求（Markdown / Excel 共用）
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ProtocolExportRequest {
    /// 报表标题
    pub title: String,
    /// 协议字段列表
    pub data: Vec<ProtocolField>,
}

/// CSV 文本解析请求（前端上传 CSV 文件内容）
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CsvParseRequest {
    /// CSV 文件文本内容（可能含 UTF-8 BOM）
    pub content: String,
}

/// 文本内容响应（Markdown 导出 / CSV 序列化共用）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TextContent {
    /// 生成的文本内容
    pub content: String,
}