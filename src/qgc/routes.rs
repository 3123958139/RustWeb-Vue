//! # QGC 飞控地面站路由（挂载于 /api/qgc）
//!
//! HTTP 端点需要认证（Bearer Token）+ `qgc:monitor` 权限；WebSocket 通过
//! `?token=` 参数在 handler 内鉴权（浏览器 WS 无法携带自定义头），因此
//! `/ws` 不挂中间件。事件通道使用 `crate::qgc::qgc_tx()` 全局广播。
//!
//! ## 路由结构
//!
//! ```text
//! /api/qgc/
//! ├── /service/start    (POST)   启动服务
//! ├── /service/stop     (POST)   停止服务
//! ├── /service/status   (GET)    查询状态
//! ├── /config           (GET)    读取配置文件
//! │                     (PUT)    保存配置文件
//! ├── /telemetry        (GET)    遥测快照
//! ├── /command          (POST)   飞控命令（arm/disarm/takeoff/land/rtl）
//! ├── /mode             (POST)   飞行模式切换
//! ├── /mission          (GET)    任务快照
//! │                     (PUT)    上传任务（航点规划）
//! │                     (DELETE) 清除任务
//! ├── /mission/download (POST)   从飞控下载任务
//! ├── /tiles/{z}/{x}/{y} (GET)   地图瓦片（代理 + 磁盘缓存，?token= 鉴权）
//! ├── /tiles/stats      (GET)    瓦片缓存统计
//! ├── /tiles/clear      (POST)   清空瓦片缓存
//! ├── /help             (GET)    帮助文档
//! └── /ws               (GET)    WebSocket 连接
//! ```

use crate::common::middleware::permission_middleware;
use crate::common::models::Permission;
use crate::database::DatabaseConnection;
use crate::qgc::handlers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    middleware::Next,
    response::Response,
    routing::{delete, get, post, put},
    Router,
};

/// 创建 QGC 飞控地面站模块的路由树
///
/// # 路由规则
/// - HTTP 端点：需要 JWT 认证 + `qgc:monitor` 权限
/// - WebSocket 端点：handler 内部通过查询参数 `?token=` 鉴权
pub fn qgc_router(db: DatabaseConnection) -> Router<DatabaseConnection> {
    let auth = middleware::from_fn_with_state(db.clone(), crate::common::middleware::auth_middleware);

    // 受 Bearer 认证保护的端点
    let protected = Router::<DatabaseConnection>::new()
        .route("/service/start", post(handlers::start_service_handler))
        .route("/service/stop", post(handlers::stop_service_handler))
        .route("/service/status", get(handlers::service_status_handler))
        .route("/config", get(handlers::read_config_handler))
        .route("/config", put(handlers::save_config_handler))
        .route("/telemetry", get(handlers::telemetry_handler))
        .route("/command", post(handlers::command_handler))
        .route("/mode", post(handlers::mode_handler))
        .route("/mission", get(handlers::get_mission_handler))
        .route("/mission", put(handlers::upload_mission_handler))
        .route("/mission", delete(handlers::clear_mission_handler))
        .route("/mission/download", post(handlers::download_mission_handler))
        .route("/tiles/stats", get(handlers::tile_stats_handler))
        .route("/tiles/clear", post(handlers::clear_tiles_handler))
        .route("/help", get(handlers::get_help_handler))
        .route_layer(middleware::from_fn_with_state(
            db.clone(),
            qgc_permission_middleware,
        ))
        .route_layer(auth);

    // 瓦片代理：Cesium 图片加载器无法携带 Bearer 头，token 经 ?token= 在 handler 内校验（同 /ws）
    // 注意：路径参数用 `:param` 冒号语法（matchit 0.7.3 的 `{param}` 花括号语法不工作；
    // OpenAPI 注解 / 文档中的 `{z}/{x}/{y}` 是 OpenAPI 标准写法，两者并行不受影响）
    let tiles = Router::<DatabaseConnection>::new()
        .route("/tiles/:z/:x/:y", get(handlers::tile_handler));

    // WebSocket：handler 内部用 ?token= 校验 JWT
    Router::<DatabaseConnection>::new()
        .route("/ws", get(handlers::ws_handler))
        .merge(protected)
        .merge(tiles)
}

/// QGC 模块权限中间件
///
/// 检查用户是否拥有 `qgc:monitor` 权限（登录 + 权限校验）
async fn qgc_permission_middleware(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    permission_middleware(Permission::QgcMonitor, State(db), request, next).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::Path;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn fake_tile(Path((z, x, y)): Path<(u32, u32, u32)>) -> &'static str {
        let _ = (z, x, y);
        "tile"
    }

    /// 验证 qgc 瓦片参数路由（matchit 0.7.3 中花括号 `{param}` 不工作，须用冒号 `:param`；
    /// 本测试同时验证参数路由与相邻静态路由（/tiles/stats、/tiles/clear）共存）
    #[tokio::test]
    async fn tiles_route_matching() {
        let router = Router::new()
            .route("/tiles/stats", get(|| async { "stats" }))
            .route("/tiles/clear", post(|| async { "clear" }))
            .route("/tiles/:z/:x/:y", get(fake_tile));
        for uri in ["/tiles/stats", "/tiles/15/27441/13387", "/tiles/5/2/1"] {
            let resp = router
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            println!("{uri}: {}", resp.status());
            assert_eq!(resp.status(), axum::http::StatusCode::OK);
        }
    }
}
