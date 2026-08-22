//! mario 角色模块 handler：超级马里奥游戏成绩 API。

use crate::common::error::AppError;
use crate::common::models::{ApiResponse, User};
use crate::database::DatabaseConnection;
use crate::mario::models::{MarioScore, MarioStats, ScoreList, SubmitScoreRequest};
use crate::mario::services::MarioService;
use axum::{
    extract::{Extension, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ScoreQuery {
    pub limit: Option<i32>,
}

/// 高分榜
#[utoipa::path(
    tag = "mario",
    get,
    path = "/api/mario/scores",
    operation_id = "marioListScores",
    params(
        ("limit" = Option<i32>, Query, description = "返回条数（默认 10，最大 50）"),
    ),
    responses(
        (status = 200, description = "高分榜单（按分数倒序）", body = ApiResponse<ScoreList>),
    ),
)]
pub async fn list_scores(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Query(query): Query<ScoreQuery>,
) -> Result<Json<ApiResponse<ScoreList>>, AppError> {
    let limit = query.limit.unwrap_or(10).clamp(1, 50);
    let result = MarioService::list_scores(&db, limit)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 提交一局游戏成绩
#[utoipa::path(
    post,
    tag = "mario",
    path = "/api/mario/scores",
    operation_id = "marioSubmitScore",
    request_body = SubmitScoreRequest,
    responses(
        (status = 200, description = "提交成功，返回写入的榜单条目（含排名）", body = ApiResponse<MarioScore>),
    ),
)]
pub async fn submit_score(
    State(db): State<DatabaseConnection>,
    Extension(user): Extension<User>,
    Json(body): Json<SubmitScoreRequest>,
) -> Result<Json<ApiResponse<MarioScore>>, AppError> {
    let result = MarioService::submit_score(&db, &user.username, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 全局统计
#[utoipa::path(
    tag = "mario",
    get,
    path = "/api/mario/stats",
    operation_id = "marioGetStats",
    responses(
        (status = 200, description = "全局统计", body = ApiResponse<MarioStats>),
    ),
)]
pub async fn get_stats(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
) -> Result<Json<ApiResponse<MarioStats>>, AppError> {
    let result = MarioService::get_stats(&db)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}