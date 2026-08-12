//! protocol_generator 角色模块 service：协议生成业务编排。
//!
//! 本模块无数据库依赖，直接转发给二级目录 `generator`（核心逻辑保持纯函数）。

use crate::protocol_generator::models::{ParameterDef, ProtocolField};
use crate::protocol_generator::protocol_generator::generator;

pub struct ProtocolGeneratorService;

impl ProtocolGeneratorService {
    /// 生成 Markdown 协议表文本
    pub fn export_markdown(title: &str, data: &[ProtocolField]) -> Result<String, String> {
        Ok(generator::export_markdown(title, data))
    }

    /// 生成 Excel 协议表二进制（xlsx）
    pub fn export_excel(title: &str, data: &[ProtocolField]) -> Result<Vec<u8>, String> {
        generator::export_excel(title, data)
    }

    /// 读取默认参数表（首次自动写入种子内容）
    pub fn load_default_csv() -> Result<Vec<ParameterDef>, String> {
        generator::load_default_csv()
    }

    /// 保存默认参数表
    pub fn save_default_csv(data: &[ParameterDef]) -> Result<(), String> {
        generator::save_default_csv(data)
    }

    /// 解析 CSV 文本为参数表（兼容 UTF-8 BOM）
    pub fn parse_csv(content: &str) -> Result<Vec<ParameterDef>, String> {
        generator::parse_csv_content(content)
    }

    /// 参数表序列化为 CSV 文本（UTF-8 BOM）
    pub fn serialize_csv(data: &[ParameterDef]) -> Result<String, String> {
        generator::serialize_csv(data)
    }
}