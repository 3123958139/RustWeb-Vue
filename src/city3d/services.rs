//! city3d 角色模块 service：城市 3D 数字孪生数据访问层。

use crate::city3d::handlers::{
    CreateBuildingRequest, CreateDistrictRequest, CreateEventRequest,
    UpdateBuildingRequest, UpdateDistrictRequest,
};
use crate::city3d::models::{
    create_result, Building, BuildingPage, CityEvent, CreateResult, District, EventPage, Overview,
    RecentEvent,
};
use crate::database::DatabaseConnection;
use uuid::Uuid;

pub struct City3dService;

impl City3dService {
    // ============ 建筑 ============

    /// 建筑列表（分页，含区域名称）
    pub async fn list_buildings(
        pool: &DatabaseConnection,
        page: i64,
        page_size: i64,
    ) -> Result<BuildingPage, Box<dyn std::error::Error>> {
        let offset = (page - 1) * page_size;
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM city3d_buildings")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query_as::<_, (Uuid, String, Uuid, String, f64, f64, f64, f64, f64, i32, String, f64, i32, f64, String, String)>(
            r#"
            SELECT b.id, b.name, b.district_id, d.name as district_name, b.x, b.z, b.width, b.depth, b.height,
                   b.floors, b.status, b.energy_kw, b.population, b.occupancy, b.created_at, b.updated_at
            FROM city3d_buildings b
            LEFT JOIN city3d_districts d ON b.district_id = d.id
            ORDER BY b.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let buildings: Vec<Building> = rows
            .into_iter()
            .map(|(id, name, district_id, district_name, x, z, width, depth, height, floors, status, energy, population, occupancy, created_at, updated_at)| {
                Building {
                    id: id.to_string(),
                    name,
                    district_id: district_id.to_string(),
                    district_name,
                    x, z,
                    width, depth, height,
                    floors,
                    status,
                    energy_kw: energy,
                    population,
                    occupancy,
                    created_at,
                    updated_at,
                }
            })
            .collect();

        Ok(BuildingPage {
            items: buildings,
            total: total.0,
            page,
            page_size,
        })
    }

    /// 创建建筑（自动写入一条事件）
    pub async fn create_building(
        pool: &DatabaseConnection,
        body: CreateBuildingRequest,
    ) -> Result<CreateResult, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO city3d_buildings (id, district_id, name, x, z, width, depth, height, floors, status, energy_kw, population, occupancy, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(id)
        .bind(body.district_id)
        .bind(&body.name)
        .bind(body.x)
        .bind(body.z)
        .bind(body.width)
        .bind(body.depth)
        .bind(body.height)
        .bind(body.floors)
        .bind(body.status.unwrap_or_else(|| "运行中".to_string()))
        .bind(body.energy_kw.unwrap_or(100.0))
        .bind(body.population.unwrap_or(1000))
        .bind(body.occupancy.unwrap_or(0.8))
        .execute(pool)
        .await?;

        // 自动写入事件
        Self::create_event_internal(
            pool,
            "info",
            &format!("新建筑落成：{}", body.name),
            &format!("{} 已竣工并投入使用，层数 {}，高度 {}m", body.name, body.floors, body.height),
        )
        .await?;

        Ok(create_result(id))
    }

    /// 更新建筑（自动写入事件）
    pub async fn update_building(
        pool: &DatabaseConnection,
        id: Uuid,
        body: UpdateBuildingRequest,
    ) -> Result<CreateResult, Box<dyn std::error::Error>> {
        // 先获取旧数据
        let old: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM city3d_buildings WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if old.is_none() {
            return Err("建筑不存在".into());
        }

        sqlx::query(
            r#"
            UPDATE city3d_buildings SET
                district_id = COALESCE($1, district_id),
                name = COALESCE($2, name),
                x = COALESCE($3, x),
                z = COALESCE($4, z),
                width = COALESCE($5, width),
                depth = COALESCE($6, depth),
                height = COALESCE($7, height),
                floors = COALESCE($8, floors),
                status = COALESCE($9, status),
                energy_kw = COALESCE($10, energy_kw),
                population = COALESCE($11, population),
                occupancy = COALESCE($12, occupancy),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $13
            "#,
        )
        .bind(body.district_id)
        .bind(&body.name)
        .bind(body.x)
        .bind(body.z)
        .bind(body.width)
        .bind(body.depth)
        .bind(body.height)
        .bind(body.floors)
        .bind(&body.status)
        .bind(body.energy_kw)
        .bind(body.population)
        .bind(body.occupancy)
        .bind(id)
        .execute(pool)
        .await?;

        let name = old.unwrap().0;
        Self::create_event_internal(
            pool,
            "info",
            &format!("建筑信息更新：{}", name),
            &format!("{} 的建筑信息已更新", name),
        )
        .await?;

        Ok(create_result(id))
    }

    /// 删除建筑（自动写入事件）
    pub async fn delete_building(
        pool: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let old: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM city3d_buildings WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if old.is_none() {
            return Err("建筑不存在".into());
        }

        sqlx::query("DELETE FROM city3d_buildings WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        let name = old.unwrap().0;
        Self::create_event_internal(
            pool,
            "warning",
            &format!("建筑拆除：{}", name),
            &format!("{} 已被拆除", name),
        )
        .await?;

        Ok(())
    }

    // ============ 区域 ============

    /// 区域列表（含建筑数量）
    ///
    /// 使用 `LEFT JOIN + GROUP BY` 一次查询出建筑数量，
    /// 避免逐区域单独 COUNT 的 N+1 查询问题。
    pub async fn list_districts(
        pool: &DatabaseConnection,
    ) -> Result<Vec<District>, Box<dyn std::error::Error>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, i32, i64, String, String)>(
            r#"
            SELECT d.id, d.name, d.code, d.color, d.description, d.sort_order,
                   COUNT(b.id) AS building_count, d.created_at, d.updated_at
            FROM city3d_districts d
            LEFT JOIN city3d_buildings b ON b.district_id = d.id
            GROUP BY d.id
            ORDER BY d.sort_order ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let result = rows
            .into_iter()
            .map(|(id, name, code, color, description, sort_order, building_count, created_at, updated_at)| District {
                id: id.to_string(),
                name,
                code,
                color,
                description,
                sort_order,
                building_count,
                created_at,
                updated_at,
            })
            .collect();

        Ok(result)
    }

    /// 创建区域
    pub async fn create_district(
        pool: &DatabaseConnection,
        body: CreateDistrictRequest,
    ) -> Result<CreateResult, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO city3d_districts (id, name, code, color, description, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(id)
        .bind(&body.name)
        .bind(&body.code)
        .bind(body.color.unwrap_or_else(|| "#00d4ff".to_string()))
        .bind(body.description.unwrap_or_default())
        .bind(body.sort_order.unwrap_or(0))
        .execute(pool)
        .await?;

        Ok(create_result(id))
    }

    /// 更新区域
    pub async fn update_district(
        pool: &DatabaseConnection,
        id: Uuid,
        body: UpdateDistrictRequest,
    ) -> Result<CreateResult, Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            UPDATE city3d_districts SET
                name = COALESCE($1, name),
                code = COALESCE($2, code),
                color = COALESCE($3, color),
                description = COALESCE($4, description),
                sort_order = COALESCE($5, sort_order),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $6
            "#,
        )
        .bind(&body.name)
        .bind(&body.code)
        .bind(&body.color)
        .bind(&body.description)
        .bind(body.sort_order)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(create_result(id))
    }

    /// 删除区域（级联删除所属建筑）
    pub async fn delete_district(
        pool: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM city3d_districts WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // ============ 事件 ============

    /// 事件列表（分页）
    pub async fn list_events(
        pool: &DatabaseConnection,
        page: i64,
        page_size: i64,
    ) -> Result<EventPage, Box<dyn std::error::Error>> {
        let offset = (page - 1) * page_size;
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM city3d_events")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
            r#"
            SELECT id, type, title, description, created_at
            FROM city3d_events
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let events: Vec<CityEvent> = rows
            .into_iter()
            .map(|(id, event_type, title, description, created_at)| CityEvent {
                id: id.to_string(),
                event_type,
                title,
                description,
                created_at,
            })
            .collect();

        Ok(EventPage {
            items: events,
            total: total.0,
            page,
            page_size,
        })
    }

    /// 创建事件
    pub async fn create_event(
        pool: &DatabaseConnection,
        body: CreateEventRequest,
    ) -> Result<CreateResult, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO city3d_events (id, type, title, description, created_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(id)
        .bind(body.r#type.unwrap_or_else(|| "info".to_string()))
        .bind(&body.title)
        .bind(body.description.unwrap_or_default())
        .execute(pool)
        .await?;

        Ok(create_result(id))
    }

    /// 删除事件
    pub async fn delete_event(
        pool: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM city3d_events WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 内部：创建事件（不通过 handler）
    async fn create_event_internal(
        pool: &DatabaseConnection,
        event_type: &str,
        title: &str,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO city3d_events (id, type, title, description, created_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(id)
        .bind(event_type)
        .bind(title)
        .bind(description)
        .execute(pool)
        .await?;
        Ok(())
    }

    // ============ 概览 ============

    /// 城市概览聚合统计
    pub async fn overview(
        pool: &DatabaseConnection,
    ) -> Result<Overview, Box<dyn std::error::Error>> {
        let total_buildings: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM city3d_buildings")
            .fetch_one(pool)
            .await?;
        let total_districts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM city3d_districts")
            .fetch_one(pool)
            .await?;
        let total_events: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM city3d_events")
            .fetch_one(pool)
            .await?;

        let total_energy: (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(energy_kw) FROM city3d_buildings",
        )
        .fetch_one(pool)
        .await?;

        let total_population: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(population) FROM city3d_buildings",
        )
        .fetch_one(pool)
        .await?;

        let active_buildings: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM city3d_buildings WHERE status = '运行中'",
        )
        .fetch_one(pool)
        .await?;

        let recent_events: Vec<(String, String, String, String)> = sqlx::query_as(
            r#"
            SELECT type, title, description, created_at
            FROM city3d_events
            ORDER BY created_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(pool)
        .await?;

        let recent: Vec<RecentEvent> = recent_events
            .into_iter()
            .map(|(t, title, desc, created)| RecentEvent {
                event_type: t,
                title,
                description: desc,
                created_at: created,
            })
            .collect();

        Ok(Overview {
            total_buildings: total_buildings.0,
            total_districts: total_districts.0,
            total_events: total_events.0,
            total_energy_kw: total_energy.0.unwrap_or(0.0),
            total_population: total_population.0.unwrap_or(0),
            active_buildings: active_buildings.0,
            recent_events: recent,
        })
    }
}
