//! # 公共设备台账（演示数据）
//!
//! fw100 / fw150 角色共用的台账演示数据。
//! 业务层返回此公共结构体，各角色在 OpenAPI 层保留自己的
//! schema 名称（`LedgerItem` / `Fw150LedgerItem`），互不影响。

use serde::Serialize;

/// 设备台账条目
///
/// fw100 直接使用本类型（schema 名 `LedgerItem`）；
/// fw150 定义同名结构的 `Fw150LedgerItem`（schema 名独立）。
#[derive(Debug, Serialize, Clone, utoipa::ToSchema)]
pub struct LedgerItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: String,
}

/// 生成当前用户的演示台账数据（与角色无关的公共逻辑）
pub fn demo_ledger_items(username: &str) -> Vec<LedgerItem> {
    vec![
        LedgerItem {
            id: "D001".to_string(),
            name: "数据采集器".to_string(),
            category: "采集设备".to_string(),
            status: "在线".to_string(),
        },
        LedgerItem {
            id: "D002".to_string(),
            name: "温度传感器".to_string(),
            category: "传感设备".to_string(),
            status: "离线".to_string(),
        },
        LedgerItem {
            id: "D003".to_string(),
            name: format!("{} 的工位终端", username),
            category: "终端设备".to_string(),
            status: "在线".to_string(),
        },
    ]
}
