use crate::handlers;
use crate::middleware::{auth_middleware, system_admin_middleware, role_middleware};
use crate::models::{Permission, UserRole};
use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use crate::database::DatabaseConnection;

pub fn create_router(db: DatabaseConnection) -> Router {
    // 公开的用户路由（不需要认证）
    let public_user_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));

    // 需要认证的用户路由
    let protected_user_routes = Router::new()
        .route("/profile", get(handlers::get_profile))
        .route("/settings", get(handlers::get_settings))
        .route("/settings", put(handlers::update_settings))
        .route("/devices", get(handlers::get_user_devices))
        .route("/devices/:id", delete(handlers::logout_device))
        .route("/export", post(handlers::export_data))
        .route("/account", delete(handlers::delete_account))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // 文章路由（需要认证）
    let post_routes = Router::new()
        .route("/", get(handlers::get_posts))
        .route("/", post(handlers::create_post))
        .route("/:id", get(handlers::get_post))
        .route("/:id", put(handlers::update_post))
        .route("/:id", delete(handlers::delete_post))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // 管理员路由
    let admin_routes = Router::new()
        .route("/", get(handlers::get_all_users))
        .route("/:id/role", put(handlers::update_user_role_admin))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), role_middleware));

    // 菜单管理路由（需要系统管理员权限）
    let menu_routes = Router::new()
        .route("/", get(handlers::get_all_menu_items))
        .route("/", post(handlers::create_menu_item))
        .route("/:id", get(handlers::get_menu_item))
        .route("/:id", put(handlers::update_menu_item))
        .route("/:id", delete(handlers::delete_menu_item))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), system_admin_middleware));

    // 角色管理路由（需要系统管理员权限）
    let role_routes = Router::new()
        .route("/", get(handlers::get_all_roles))
        .route("/", post(handlers::create_role))
        .route("/:id", get(handlers::get_role))
        .route("/:id", put(handlers::update_role))
        .route("/:id", delete(handlers::delete_role))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware))
        .route_layer(middleware::from_fn_with_state(db.clone(), system_admin_middleware));

    // 主路由
    Router::new()
        .route("/health", get(handlers::health_check))
        .nest("/api/users", public_user_routes.merge(protected_user_routes).merge(admin_routes))
        .nest("/api/posts", post_routes)
        .nest("/api/menu", menu_routes)
        .nest("/api/roles", role_routes)
        .with_state(db)
}
