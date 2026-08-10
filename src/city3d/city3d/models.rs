//! city3d 角色模块数据模型（DTO）。
//!
//! 供 OpenAPI 文档（utoipa）与前端类型生成使用，
//! 字段与 services 层实际返回的 JSON 保持一一对应。

use serde::Serialize;
use uuid::Uuid;

/// 建筑（列表项，含区域名称）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub district_id: String,
    pub district_name: String,
    pub x: f64,
    pub z: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub floors: i32,
    pub status: String,
    pub energy_kw: f64,
    pub population: i32,
    pub occupancy: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// 建筑列表分页
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BuildingPage {
    pub items: Vec<Building>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 区域（含建筑数量）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct District {
    pub id: String,
    pub name: String,
    pub code: String,
    pub color: String,
    pub description: String,
    pub sort_order: i32,
    pub building_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 事件
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CityEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
}

/// 事件列表分页
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EventPage {
    pub items: Vec<CityEvent>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 创建/更新操作结果（返回新生成的 id）
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResult {
    pub id: String,
}

/// 概览中的最近事件
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecentEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
}

/// 城市概览聚合统计
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Overview {
    pub total_buildings: i64,
    pub total_districts: i64,
    pub total_events: i64,
    pub total_energy_kw: f64,
    pub total_population: i64,
    pub active_buildings: i64,
    pub recent_events: Vec<RecentEvent>,
}

/// 从 UUID 生成 id 字符串的结果
pub fn create_result(id: Uuid) -> CreateResult {
    CreateResult { id: id.to_string() }
}
