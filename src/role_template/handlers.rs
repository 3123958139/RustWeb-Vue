//! 角色模块模板 handler 示例。

use crate::common::error::AppError;
use crate::common::models::{ApiResponse, User};
use crate::role_template::services::TemplateService;
use axum::{
    extract::{Extension, State},
    Json,
};
use crate::database::DatabaseConnection;

/// 示例接口：返回当前用户可见的模板数据列表（登录 + 权限中间件保护）
pub async fn list_template_items(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
    let items = TemplateService::list_items(&db, &user).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(items)))
}
