use crate::services::UserService;
use crate::utils::{jwt, AppError};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use crate::database::DatabaseConnection;

pub async fn auth_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = auth_header.ok_or_else(|| AppError::unauthorized("缺少认证token"))?;

    let user_id = jwt::verify_token(token)?;
    let user = UserService::get_user_by_id(&db, user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::unauthorized("用户不存在"))?;

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        if let Ok(user_id) = jwt::verify_token(token) {
            if let Ok(Some(user)) = UserService::get_user_by_id(&db, user_id).await {
                request.extensions_mut().insert(user);
            }
        }
    }

    next.run(request).await
}
