//! # RustWeb-Vue 后端服务器入口
//!
//! 本模块是整个后端应用的启动入口，负责：
//! 1. 初始化日志系统
//! 2. 加载配置（环境变量）
//! 3. 初始化数据库连接
//! 4. 配置 CORS（跨域资源共享）
//! 5. 注册所有 API 路由
//! 6. 配置静态文件服务（生产环境）
//! 7. 启动 HTTP 服务器
//!
//! # 架构说明
//!
//! ```text
//! 客户端（Vue 前端）
//!     ↓ HTTP 请求
//! Axum 服务器（本文件启动）
//!     ↓ 路由匹配
//! 中间件（CORS、认证）
//!     ↓
//! 处理器（handlers）
//!     ↓
//! 服务层（services）
//!     ↓
//! 数据库（SQLite）
//! ```
//!
//! # 启动流程
//!
//! 1. `dotenv::dotenv()` - 加载 `.env` 文件中的环境变量
//! 2. `tracing_subscriber` - 初始化日志系统（支持 `RUST_LOG` 环境变量控制日志级别）
//! 3. `AppConfig::load()` - 从环境变量读取配置（端口、数据库URL等）
//! 4. `init_database()` - 创建数据库连接池并初始化表结构
//! 5. `create_router()` - 注册所有 API 路由
//! 6. 配置 CORS 和静态文件服务
//! 7. 绑定 TCP 端口并启动服务器

// ============ 模块声明 ============
// Rust 使用 `mod` 关键字声明模块，模块树结构对应文件系统目录结构
// 例如：`mod admin;` 对应 `src/admin/mod.rs` 或 `src/admin.rs`
mod admin; // 管理员模块：用户管理、角色管理
mod api_docs; // OpenAPI 文档定义与导出
mod city3d; // city3d 角色模块：城市 3D 展示
mod common; // 公共模块：认证、中间件、数据模型、错误处理
mod config; // 配置模块：从环境变量加载应用配置
mod database; // 数据库模块：SQLite 连接、表创建、种子数据
mod fj200c_information; // fj200c_information 角色模块：发动机监控
mod fj200c_main; // fj200c_main 角色模块：发动机测控（ECU/Adam4015/Adam4117/Dyno/Flux 五路串口）
mod ftj1c; // ftj1c 角色模块：UDP 组播通信监控
mod fw100; // fw150 角色模块：设备台账管理
mod fw150;
mod protocol_generator; // protocol_generator 角色模块：通信协议生成器
mod qgc; // qgc 角色模块：飞控地面站（MAVLink v2 + 模拟飞控）
mod role_template; // 角色模板：新角色开发的参考模板
mod roles; // 角色注册表：全系统角色定义的单一事实来源
mod routes; // 路由模块：集中注册所有 API 路由

// 单 exe 打包：`--features embedded` 时嵌入 6 个前端 dist，运行时从内存服务
#[cfg(feature = "embedded")]
mod embedded_assets;

// ============ 导入依赖 ============
// `use` 关键字用于引入其他模块/包中的类型、函数等
// `axum` 是一个高性能的 Web 框架，类似 Express.js（Node.js）或 Flask（Python）
use axum::{
    http::{HeaderValue, Method}, // HTTP 方法枚举（GET、POST、PUT、DELETE 等）
    middleware::{self, Next},    // 自定义中间件
    response::{IntoResponse, Redirect}, // HTTP 重定向响应
    routing::get,                // 路由函数：注册 GET 请求处理器
};
use std::net::SocketAddr;
// 网络地址类型（IP + 端口）
// `tower_http` 提供 HTTP 中间件，CORS 用于处理跨域请求
use tower_http::cors::{Any, CorsLayer};
// CORS 中间件
// 请求日志层
use tower_http::trace::TraceLayer;
// 静态文件服务（仅 dev 模式使用；`embedded` feature 下改用内存嵌入，见 embedded_assets.rs）
#[cfg(not(feature = "embedded"))]
use tower_http::services::{ServeDir, ServeFile};
// `tracing_subscriber` 是日志框架，用于输出结构化日志
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 引入本项目的配置、数据库初始化、路由创建函数
// `crate` 关键字表示当前 crate（包）的根模块
use crate::config::AppConfig;
use crate::database::init_database;
use crate::routes::create_router;

// ============ 主函数 ============
// `#[tokio::main]` 是一个过程宏（proc macro），它将 `async fn main()` 转换为
// 标准的同步 `fn main()`，并在内部启动 Tokio 异步运行时（runtime）
//
// Tokio 是 Rust 最流行的异步运行时，提供：
// - 异步任务调度
// - 异步 I/O（网络、文件）
// - 定时器、通道等异步原语
//
// `async fn` 表示这是一个异步函数，可以使用 `.await` 等待异步操作完成
// 返回值 `Result<(), Box<dyn std::error::Error>>` 表示：
// - `Ok(())` - 成功返回空值
// - `Err(...)` - 失败返回错误（`Box<dyn Error>` 是一个类型擦除的错误容器）
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ============ 1. 加载环境变量 ============
    // `dotenvy::dotenv()` 读取项目根目录的 `.env` 文件，将其中的键值对设置为环境变量
    // `.ok()` 将 `Result` 转换为 `Option`，忽略错误（文件不存在时静默失败）
    // 例如：`.env` 文件中 `PORT=3000` 会被设置为环境变量
    dotenvy::dotenv().ok();

    // ============ 2. 初始化日志系统 ============
    // `tracing_subscriber` 提供结构化日志，支持：
    // - 日志级别过滤（info、debug、warn、error）
    // - 格式化输出（时间、模块、消息）
    // - 多种输出目标（控制台、文件、网络）
    //
    // `EnvFilter` 从 `RUST_LOG` 环境变量读取过滤规则
    // 例如：`RUST_LOG=debug` 显示所有日志，`RUST_LOG=axum=debug` 只显示 axum 的 debug 日志
    // 默认级别为 `info`（显示 info 及以上级别）
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer()) // 格式化输出到控制台
        .init(); // 初始化全局日志订阅者

    // ============ 3. 加载应用配置 ============
    // `AppConfig::load()` 从环境变量读取配置：
    // - `PORT`: 服务器端口（默认 3000）
    // - `DATABASE_URL`: 数据库连接 URL（默认 `sqlite://rustweb.db`）
    //
    // `?` 操作符是 Rust 的错误传播语法：
    // - 如果 `Result` 是 `Ok`，解包返回值
    // - 如果是 `Err`，提前返回该错误（类似 Go 的 `if err != nil { return err }`）
    let config = AppConfig::load()?;

    // ============ 4. 初始化数据库 ============
    // `init_database()` 创建 SQLite 连接池，创建表结构，插入种子数据
    // 返回 `SqlitePool`（SQLx 的连接池类型），支持并发数据库操作
    //
    // `.await` 等待异步操作完成（类似 JavaScript 的 `await`、Python 的 `await`）
    // 但 Rust 的 `.await` 是编译器级别的，性能更高（无回调地狱）
    let pool = init_database(&config.database_url).await?;

    // ============ 4.5 初始化 JWT 配置 ============
    // 缓存签名密钥与过期时间（OnceLock），生产模式缺失 JWT_SECRET 直接拒绝启动
    crate::common::jwt::init()?;

    // ============ 5. 配置 CORS（跨域资源共享） ============
    // 当前端（如 `http://localhost:5173`）请求后端（`http://localhost:3000`）时，
    // 浏览器会检查 CORS 头，决定是否允许请求
    //
    // `CorsLayer` 配置允许的：
    // - HTTP 方法：GET、POST、PUT、DELETE
    // - 请求头：任何（`Any`）
    // - 来源：dev 模式任何（`Any`）；生产模式必须通过 `CORS_ORIGINS` 白名单（逗号分隔）
    let cors = {
        let methods = [Method::GET, Method::POST, Method::PUT, Method::DELETE];
        if cfg!(debug_assertions) {
            CorsLayer::new()
                .allow_methods(methods)
                .allow_headers(Any)
                .allow_origin(Any)
        } else {
            let origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<HeaderValue>().ok())
                .collect();
            if origins.is_empty() {
                return Err(
                    "生产模式必须设置 CORS_ORIGINS 环境变量（逗号分隔的前端来源白名单，如 http://192.168.1.10:3000）"
                        .into(),
                );
            }
            CorsLayer::new()
                .allow_methods(methods)
                .allow_headers(Any)
                .allow_origin(origins)
        }
    };

    // ============ 6. 创建路由和静态文件服务 ============
    // `create_router()` 注册所有 API 路由（如 `/api/auth/login`、`/api/users` 等）
    //
    // `.layer(cors)` 添加 CORS 中间件（所有请求都会经过此中间件）
    //
    // `.route("/", ...)` 注册根路径 `/` 的处理器：
    // - 访问 `/` 会永久重定向到 `/admin`
    //
    // 静态托管分两种模式（由 Cargo feature 决定，见 Cargo.toml）：
    // - `--features embedded`（单 exe 打包，deploy.bat 默认）：
    //   6 个前端 dist 编译期内嵌进二进制，`embedded_assets::embedded_router()`
    //   从内存直接返回字节，磁盘上不再需要 dist-* 目录
    // - 默认（dev 模式）：`.nest_service()` 嵌套磁盘静态文件服务，
    //   `/admin` → `dist-admin/` 目录，其余 5 个应用同理
    //
    // `ServeDir` 提供目录下的静态文件
    // `fallback(ServeFile::new(...))` 是 SPA（单页应用）的深链接支持：
    // - 当请求的文件不存在时，返回 `index.html`
    // - 这样 Vue Router 的 history 模式才能正常工作
    //
    // 注意：`.layer()` 在 axum 0.7 中只作用于调用时**已注册**的路由，
    // 因此 CORS 中间件放在最后，确保 API 路由与嵌入式静态路由都被包裹
    let app = {
        let base = create_router(pool)
            .layer(cors)
            .route("/", get(|| async { Redirect::temporary("/admin") }));

        #[cfg(feature = "embedded")]
        let app = base.merge(embedded_assets::embedded_router());

        #[cfg(not(feature = "embedded"))]
        let app = base
            .nest_service(
                "/admin",
                ServeDir::new("dist-admin").fallback(ServeFile::new("dist-admin/index.html")),
            )
            .nest_service(
                "/fw100",
                ServeDir::new("dist-fw100").fallback(ServeFile::new("dist-fw100/index.html")),
            )
            .nest_service(
                "/fj200c_information",
                ServeDir::new("dist-fj200c_information").fallback(ServeFile::new("dist-fj200c_information/index.html")),
            )
            .nest_service(
                "/fj200c_main",
                ServeDir::new("dist-fj200c_main")
                    .fallback(ServeFile::new("dist-fj200c_main/index.html")),
            )
            .nest_service(
                "/ftj1c",
                ServeDir::new("dist-ftj1c").fallback(ServeFile::new("dist-ftj1c/index.html")),
            )
            .nest_service(
                "/city3d",
                ServeDir::new("dist-city3d").fallback(ServeFile::new("dist-city3d/index.html")),
            )
            .nest_service(
                "/fw150",
                ServeDir::new("dist-fw150").fallback(ServeFile::new("dist-fw150/index.html")),
            )
            .nest_service(
                "/protocol_generator",
                ServeDir::new("dist-protocol_generator")
                    .fallback(ServeFile::new("dist-protocol_generator/index.html")),
            )
            .nest_service(
                "/qgc",
                ServeDir::new("dist-qgc").fallback(ServeFile::new("dist-qgc/index.html")),
            );

        app
    };

    // 静态资源缓存头：Vite 构建产物（`assets/` 下的 hash 文件名）永久缓存；
    // HTML 入口不缓存（no-cache），保证发版后立即生效
    async fn static_cache_headers(request: axum::extract::Request, next: Next) -> impl IntoResponse {
        let path = request.uri().path().to_string();
        let mut response = next.run(request).await;
        if path.contains("/assets/") {
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        } else if path.ends_with(".html") {
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("no-cache"),
            );
        }
        response
    }

    // 请求日志（tower-http TraceLayer）+ 静态缓存头中间件
    let app = app
        .layer(middleware::from_fn(static_cache_headers))
        .layer(TraceLayer::new_for_http());

    // ============ 7. 启动服务器 ============
    // `SocketAddr` 表示网络地址，格式为 `IP:端口`
    // `127.0.0.1` 是本地回环地址（localhost），只允许本机访问
    // 如需允许外部访问，使用 `0.0.0.0`
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("服务器启动在 http://{}", addr);

    // 创建 TCP 监听器，绑定到指定地址
    // `tokio::net::TcpListener` 是异步 TCP 监听器
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // 启动 HTTP 服务器，处理所有传入的连接
    // `into_make_service_with_connect_info` 让处理器可通过 `ConnectInfo` 获取客户端 IP
    // （登录速率限制按 IP 计数）
    // `with_graceful_shutdown` 在 Ctrl+C / 系统关闭信号时优雅退出：
    // 不再接受新连接，等待在途请求完成（CSV 不丢尾帧、WS 收关闭帧）
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// 等待关闭信号（Ctrl+C 或系统终止信号）
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("无法注册 Ctrl+C 信号处理器");
    };

    #[cfg(windows)]
    let terminate = async {
        match tokio::signal::windows::ctrl_shutdown() {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => tracing::warn!("无法注册关闭信号处理器: {}", e),
        }
    };

    #[cfg(not(windows))]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("无法注册终止信号处理器")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("收到关闭信号，正在优雅退出…");
}
