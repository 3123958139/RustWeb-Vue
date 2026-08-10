//! fw100 角色模块 handler：设备台账查询。

use crate::common::error::AppError;
use crate::common::ledger::LedgerItem;
use crate::common::models::{ApiResponse, User};
use crate::database::DatabaseConnection;
use crate::fw100::services::Fw100Service;
use axum::{
    extract::{Extension, State},
    Json,
};

/// 设备台账列表（登录 + fw100:read 权限中间件保护）
#[utoipa::path(
    tag = "fw100",
    get,
    path = "/api/fw100/items",
    operation_id = "fw100ListItems",
    responses(
        (status = 200, description = "设备台账列表", body = ApiResponse<Vec<LedgerItem>>),
    ),
)]
pub async fn list_fw100_items(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<Vec<LedgerItem>>>, AppError> {
    let items = Fw100Service::list_items(&db, &user)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(items)))
}
