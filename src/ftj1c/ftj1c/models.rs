//! ftj1c 角色模块数据模型（DTO）。
//!
//! 供 OpenAPI 文档（utoipa）与前端类型生成使用，
//! 字段与 handlers 层实际返回的 JSON 保持一一对应。

use serde::{Deserialize, Serialize};

/// IP 配置（config-ftj1c.ini 的 [IP] 节，16 组 IP:Port 配对）
///
/// 运行时实际返回包含全部 32 个键的动态对象，
/// 这里仅作为 OpenAPI schema 描述，键均视为可选。
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IpConfig {
    pub ip1: Option<String>,
    pub port1: Option<u16>,
    pub ip2: Option<String>,
    pub port2: Option<u16>,
    pub ip3: Option<String>,
    pub port3: Option<u16>,
    pub ip4: Option<String>,
    pub port4: Option<u16>,
    pub ip5: Option<String>,
    pub port5: Option<u16>,
    pub ip6: Option<String>,
    pub port6: Option<u16>,
    pub ip7: Option<String>,
    pub port7: Option<u16>,
    pub ip8: Option<String>,
    pub port8: Option<u16>,
    pub ip9: Option<String>,
    pub port9: Option<u16>,
    pub ip10: Option<String>,
    pub port10: Option<u16>,
    pub ip11: Option<String>,
    pub port11: Option<u16>,
    pub ip12: Option<String>,
    pub port12: Option<u16>,
    pub ip13: Option<String>,
    pub port13: Option<u16>,
    pub ip14: Option<String>,
    pub port14: Option<u16>,
    pub ip15: Option<String>,
    pub port15: Option<u16>,
    pub ip16: Option<String>,
    pub port16: Option<u16>,
}

/// 保存配置请求体结构
///
/// # 示例 JSON
/// ```json
/// {
///   "content": "[Udp]\nMock = true\n[IP]\nip1 = 192.168.1.1\n..."
/// }
/// ```
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct Ftj1cSaveConfigRequest {
    /// INI 配置文件的完整内容
    pub content: String,
}
