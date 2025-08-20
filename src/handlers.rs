use crate::models::{
    ApiResponse, CreatePostRequest, CreateUserRequest, LoginRequest, LoginResponse, Post, User,
};
use crate::services::{PostService, UserService};
use crate::utils::{jwt, AppError};
use axum::{
    extract::{Path, State, Extension},
    http::StatusCode,
    Json,
};
use crate::database::DatabaseConnection;
use uuid::Uuid;

use validator::Validate;

// 用户相关处理器
pub async fn register(
    State(db): State<DatabaseConnection>,
    Json(user_data): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    user_data.validate()?;
    
    let user = UserService::create_user(&db, user_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(user)))
}

pub async fn login(
    State(db): State<DatabaseConnection>,
    Json(login_data): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    login_data.validate()?;
    
    let user = UserService::login(&db, login_data).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let token = jwt::create_token(&user)?;
    
    let response = LoginResponse { token, user };
    Ok(Json(ApiResponse::success(response)))
}

pub async fn get_profile(
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    Ok(Json(ApiResponse::success(user)))
}

// 文章相关处理器
pub async fn create_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(post_data): Json<CreatePostRequest>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    post_data.validate()?;
    
    let post = PostService::create_post(&db, post_data, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(post)))
}

pub async fn get_posts(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<Post>>>, AppError> {
    let posts = PostService::get_posts(&db).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(posts)))
}

pub async fn get_post(
    State(db): State<DatabaseConnection>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    let post = PostService::get_post_by_id(&db, post_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("文章不存在"))?;
    
    Ok(Json(ApiResponse::success(post)))
}

pub async fn update_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Path(post_id): Path<Uuid>,
    Json(post_data): Json<CreatePostRequest>,
) -> Result<Json<ApiResponse<Post>>, AppError> {
    post_data.validate()?;
    
    let post = PostService::update_post(&db, post_id, post_data, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(post)))
}

pub async fn delete_post(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    PostService::delete_post(&db, post_id, user.id).await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// 健康检查
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
