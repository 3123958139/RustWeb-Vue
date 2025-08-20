use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use crate::{
    models::{User, Permission, UserRole},
    services::UserService,
    utils::jwt,
    database::DatabaseConnection,
};

pub async fn auth_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "));

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = jwt::verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = UserService::get_user_by_id(&db, user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        if let Ok(user_id) = jwt::verify_token(token) {
            if let Ok(Some(user)) = UserService::get_user_by_id(&db, user_id).await {
                request.extensions_mut().insert(user);
            }
        }
    }

    Ok(next.run(request).await)
}

// 权限验证中间件
pub async fn permission_middleware(
    required_permission: Permission,
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "));

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = jwt::verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = UserService::get_user_by_id(&db, user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !user.has_permission(&required_permission) {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

// 系统管理员权限中间件
pub async fn system_admin_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::SystemAdmin, State(db), request, next).await
}

// 角色验证中间件
pub async fn role_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "));

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = jwt::verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = UserService::get_user_by_id(&db, user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user_role = user.get_role();
    
    // 检查角色权限等级 - 这里硬编码为Admin权限检查
    let has_permission = match user_role {
        UserRole::Admin => true,
        UserRole::Moderator => false,
        UserRole::User => false,
    };

    if !has_permission {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

// 错误响应辅助函数
pub fn forbidden_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "success": false,
            "message": "权限不足"
        })),
    )
}

pub fn unauthorized_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "success": false,
            "message": "未授权访问"
        })),
    )
}
