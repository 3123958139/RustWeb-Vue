//! fw150 角色模块 service：设备台账（演示数据，验证角色解耦）。

use crate::common::ledger::demo_ledger_items;
use crate::common::models::User;
use crate::database::DatabaseConnection;
use serde::Serialize;

/// 设备台账条目（fw150 独立 schema 名，与 fw100 的 `LedgerItem` 区分）
#[derive(Debug, Serialize, Clone, utoipa::ToSchema)]
pub struct Fw150LedgerItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: String,
}

pub struct Fw150Service;

impl Fw150Service {
    /// 设备台账列表：返回当前用户可见的台账数据（演示）
    pub async fn list_items(
        _pool: &DatabaseConnection,
        user: &User,
    ) -> Result<Vec<Fw150LedgerItem>, Box<dyn std::error::Error>> {
        Ok(demo_ledger_items(&user.username)
            .into_iter()
            .map(|item| Fw150LedgerItem {
                id: item.id,
                name: item.name,
                category: item.category,
                status: item.status,
            })
            .collect())
    }
}
