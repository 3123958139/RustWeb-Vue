//! city3d 角色模块 handler：城市 3D 数字孪生 API。

use crate::city3d::models::{
    BuildingPage, CreateResult, District, EventPage, Overview,
};
use crate::city3d::services::City3dService;
use crate::common::error::AppError;
use crate::common::models::{ApiResponse, User};
use crate::database::DatabaseConnection;
use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

// ============ 请求体 ============

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateBuildingRequest {
    pub district_id: Uuid,
    pub name: String,
    pub x: f64,
    pub z: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub floors: i32,
    pub status: Option<String>,
    pub energy_kw: Option<f64>,
    pub population: Option<i32>,
    pub occupancy: Option<f64>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateBuildingRequest {
    pub district_id: Option<Uuid>,
    pub name: Option<String>,
    pub x: Option<f64>,
    pub z: Option<f64>,
    pub width: Option<f64>,
    pub depth: Option<f64>,
    pub height: Option<f64>,
    pub floors: Option<i32>,
    pub status: Option<String>,
    pub energy_kw: Option<f64>,
    pub population: Option<i32>,
    pub occupancy: Option<f64>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateDistrictRequest {
    pub name: String,
    pub code: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateDistrictRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateEventRequest {
    pub r#type: Option<String>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ============ 建筑 ============

/// 建筑列表
#[utoipa::path(
    tag = "city3d",
    get,
    path = "/api/city3d/buildings",
    operation_id = "city3dListBuildings",
    params(
        ("page" = Option<i64>, Query, description = "页码（默认 1）"),
        ("page_size" = Option<i64>, Query, description = "每页数量（默认 50，最大 200）"),
    ),
    responses(
        (status = 200, description = "建筑列表（分页）", body = ApiResponse<BuildingPage>),
    ),
)]
pub async fn list_buildings(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<BuildingPage>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).min(200);
    let result = City3dService::list_buildings(&db, page, page_size)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 创建建筑
#[utoipa::path(
    post,
    tag = "city3d",
    path = "/api/city3d/buildings",
    operation_id = "city3dCreateBuilding",
    request_body = CreateBuildingRequest,
    responses(
        (status = 200, description = "创建成功，返回新建筑 id", body = ApiResponse<CreateResult>),
    ),
)]
pub async fn create_building(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Json(body): Json<CreateBuildingRequest>,
) -> Result<Json<ApiResponse<CreateResult>>, AppError> {
    let result = City3dService::create_building(&db, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 更新建筑
#[utoipa::path(
    put,
    tag = "city3d",
    path = "/api/city3d/buildings/{id}",
    operation_id = "city3dUpdateBuilding",
    request_body = UpdateBuildingRequest,
    params(
        ("id" = Uuid, Path, description = "建筑 UUID"),
    ),
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<CreateResult>),
    ),
)]
pub async fn update_building(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBuildingRequest>,
) -> Result<Json<ApiResponse<CreateResult>>, AppError> {
    let result = City3dService::update_building(&db, id, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 删除建筑
#[utoipa::path(
    tag = "city3d",
    delete,
    path = "/api/city3d/buildings/{id}",
    operation_id = "city3dDeleteBuilding",
    params(
        ("id" = Uuid, Path, description = "建筑 UUID"),
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<serde_json::Value>),
    ),
)]
pub async fn delete_building(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    City3dService::delete_building(&db, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// ============ 区域 ============

/// 区域列表
#[utoipa::path(
    tag = "city3d",
    get,
    path = "/api/city3d/districts",
    operation_id = "city3dListDistricts",
    responses(
        (status = 200, description = "区域列表（含建筑数量）", body = ApiResponse<Vec<District>>),
    ),
)]
pub async fn list_districts(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
) -> Result<Json<ApiResponse<Vec<District>>>, AppError> {
    let result = City3dService::list_districts(&db)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 创建区域
#[utoipa::path(
    post,
    tag = "city3d",
    path = "/api/city3d/districts",
    operation_id = "city3dCreateDistrict",
    request_body = CreateDistrictRequest,
    responses(
        (status = 200, description = "创建成功，返回新区域 id", body = ApiResponse<CreateResult>),
    ),
)]
pub async fn create_district(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Json(body): Json<CreateDistrictRequest>,
) -> Result<Json<ApiResponse<CreateResult>>, AppError> {
    let result = City3dService::create_district(&db, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 更新区域
#[utoipa::path(
    put,
    tag = "city3d",
    path = "/api/city3d/districts/{id}",
    operation_id = "city3dUpdateDistrict",
    request_body = UpdateDistrictRequest,
    params(
        ("id" = Uuid, Path, description = "区域 UUID"),
    ),
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<CreateResult>),
    ),
)]
pub async fn update_district(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDistrictRequest>,
) -> Result<Json<ApiResponse<CreateResult>>, AppError> {
    let result = City3dService::update_district(&db, id, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 删除区域（级联删除所属建筑）
#[utoipa::path(
    tag = "city3d",
    delete,
    path = "/api/city3d/districts/{id}",
    operation_id = "city3dDeleteDistrict",
    params(
        ("id" = Uuid, Path, description = "区域 UUID"),
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<serde_json::Value>),
    ),
)]
pub async fn delete_district(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    City3dService::delete_district(&db, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// ============ 事件 ============

/// 事件列表
#[utoipa::path(
    tag = "city3d",
    get,
    path = "/api/city3d/events",
    operation_id = "city3dListEvents",
    params(
        ("page" = Option<i64>, Query, description = "页码（默认 1）"),
        ("page_size" = Option<i64>, Query, description = "每页数量（默认 20，最大 100）"),
    ),
    responses(
        (status = 200, description = "事件列表（分页）", body = ApiResponse<EventPage>),
    ),
)]
pub async fn list_events(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<EventPage>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let result = City3dService::list_events(&db, page, page_size)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 创建事件
#[utoipa::path(
    post,
    tag = "city3d",
    path = "/api/city3d/events",
    operation_id = "city3dCreateEvent",
    request_body = CreateEventRequest,
    responses(
        (status = 200, description = "创建成功，返回新事件 id", body = ApiResponse<CreateResult>),
    ),
)]
pub async fn create_event(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Json(body): Json<CreateEventRequest>,
) -> Result<Json<ApiResponse<CreateResult>>, AppError> {
    let result = City3dService::create_event(&db, body)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 删除事件
#[utoipa::path(
    tag = "city3d",
    delete,
    path = "/api/city3d/events/{id}",
    operation_id = "city3dDeleteEvent",
    params(
        ("id" = Uuid, Path, description = "事件 UUID"),
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<serde_json::Value>),
    ),
)]
pub async fn delete_event(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    City3dService::delete_event(&db, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

// ============ 概览 ============

/// 城市概览聚合统计
#[utoipa::path(
    tag = "city3d",
    get,
    path = "/api/city3d/overview",
    operation_id = "city3dGetOverview",
    responses(
        (status = 200, description = "城市概览统计", body = ApiResponse<Overview>),
    ),
)]
pub async fn overview(
    State(db): State<DatabaseConnection>,
    Extension(_user): Extension<User>,
) -> Result<Json<ApiResponse<Overview>>, AppError> {
    let result = City3dService::overview(&db)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}
