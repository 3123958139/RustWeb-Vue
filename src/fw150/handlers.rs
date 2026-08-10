//! fw150 角色模块 handler：设备台账查询。

use crate::common::error::AppError;
use crate::common::models::{ApiResponse, User};
use crate::database::DatabaseConnection;
use crate::fw150::services::{Fw150LedgerItem, Fw150Service};
use axum::{
    extract::{Extension, State},
    Json,
};

/// 设备台账列表（登录 + fw150:read 权限中间件保护）
#[utoipa::path(
    tag = "fw150",
    get,
    path = "/api/fw150/items",
    operation_id = "fw150ListItems",
    responses(
        (status = 200, description = "设备台账列表", body = ApiResponse<Vec<Fw150LedgerItem>>),
    ),
)]
pub async fn list_fw150_items(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<Vec<Fw150LedgerItem>>>, AppError> {
    let items = Fw150Service::list_items(&db, &user)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(items)))
}
