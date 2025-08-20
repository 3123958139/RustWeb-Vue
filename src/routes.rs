use crate::handlers;
use crate::middleware::auth_middleware;
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
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // 文章路由（需要认证）
    let post_routes = Router::new()
        .route("/", get(handlers::get_posts))
        .route("/", post(handlers::create_post))
        .route("/:id", get(handlers::get_post))
        .route("/:id", put(handlers::update_post))
        .route("/:id", delete(handlers::delete_post))
        .route_layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // 主路由
    Router::new()
        .route("/health", get(handlers::health_check))
        .nest("/api/users", public_user_routes.merge(protected_user_routes))
        .nest("/api/posts", post_routes)
        .with_state(db)
}
