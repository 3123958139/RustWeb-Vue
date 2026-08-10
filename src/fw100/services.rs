//! fw100 角色模块 service：设备台账（演示数据，验证角色解耦）。

use crate::common::ledger::{demo_ledger_items, LedgerItem};
use crate::common::models::User;
use crate::database::DatabaseConnection;

pub struct Fw100Service;

impl Fw100Service {
    /// 设备台账列表：返回当前用户可见的台账数据（演示）
    pub async fn list_items(
        _pool: &DatabaseConnection,
        user: &User,
    ) -> Result<Vec<LedgerItem>, Box<dyn std::error::Error>> {
        Ok(demo_ledger_items(&user.username))
    }
}
