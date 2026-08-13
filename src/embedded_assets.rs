//! # 前端静态资源嵌入模块（`--features embedded` 编译）
//!
//! 打包单 exe 时，将 7 个前端应用的 `frontend/<app>/dist/` 构建产物
//! 在编译期内嵌进二进制（rust-embed），运行时直接从内存返回字节，
//! 不再依赖磁盘上的 `dist-*` 目录。
//!
//! 本模块仅在启用 `embedded` feature 时编译（见 `Cargo.toml`）：
//! - `cargo build --release --features embedded` → 单 exe 部署产物
//! - 默认 `cargo run` / `cargo test` 不编译本模块，继续使用 `ServeDir`（dev 模式）

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

// ============ 各应用静态资源 ============
// `#[folder]` 路径相对于 crate 根目录（Cargo.toml 所在位置），
// 编译期要求目录存在（deploy.bat 先构建前端再编译后端可保证）

/// 批量生成各应用的 rust-embed 资源结构体
///
/// 每个应用一个 `#[derive(RustEmbed)]` 结构体，folder 指向 `frontend/<app>/dist/`。
/// 新增前端应用时只需在此追加一项（同时需在 `embedded_router` 注册路由前缀）。
macro_rules! embed_assets {
    ($($struct_name:ident => $folder:literal),+ $(,)?) => {
        $(
            #[derive(RustEmbed)]
            #[folder = $folder]
            struct $struct_name;
        )+
    };
}

embed_assets!(
    AdminAssets => "frontend/admin/dist/",
    Fj200cInformationAssets => "frontend/fj200c_information/dist/",
    Fj200cMainAssets => "frontend/fj200c_main/dist/",
    Fw100Assets => "frontend/fw100/dist/",
    Fw150Assets => "frontend/fw150/dist/",
    Ftj1cAssets => "frontend/ftj1c/dist/",
    City3dAssets => "frontend/city3d/dist/",
    ProtocolGeneratorAssets => "frontend/protocol_generator/dist/",
);

// ============ 处理函数 ============

/// 嵌入式静态资源处理器（泛型，按应用实例化）
///
/// 行为与原 `ServeDir.fallback(ServeFile::new(index.html))` 一致：
/// 1. 空路径（如 `/admin`）→ 返回 `index.html`
/// 2. 命中资源 → 返回字节 + 按扩展名推断的 Content-Type
/// 3. 未命中 → 回退到该应用的 `index.html`（SPA 深链接），
///    且 MIME 按实际返回的 `index.html` 计算（避免 octet-stream）
/// 4. 连回退都不存在 → 404
///
/// `Option<Path<String>>`：`/admin` 精确路由无路径参数时为 `None`，
/// `/admin/*path` 通配路由时为 `Some`，两者共用同一处理器
async fn serve_embedded<A: RustEmbed>(path: Option<Path<String>>) -> Response {
    let path = path.map(|p| p.0).unwrap_or_default();
    let file_path = if path.is_empty() {
        "index.html"
    } else {
        path.as_str()
    };

    let asset = match A::get(file_path) {
        Some(content) => Some((file_path, content)),
        None => A::get("index.html").map(|content| ("index.html", content)),
    };

    match asset {
        Some((mime_key, content)) => {
            let mime = from_path(mime_key).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data,
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ============ 路由组装 ============

/// 批量注册嵌入式静态资源路由
///
/// 每个应用注册三条路由（axum 0.7 通配符语法 `/*path`）：
/// - `/x`（精确，无斜杠）→ `Option<Path>` 为 `None`，返回 index.html
/// - `/x/`（精确，带尾斜杠）→ 同上；缺少它时 matchit 会对 `/x/`
///   触发 ExtraTrailingSlash 检查返回 404（已有实测验证）
/// - `/x/*path`（通配，含任意深层路径）→ 资源命中或 SPA 回退
macro_rules! embed_app_routes {
    ($router:expr, $($prefix:literal => $assets:ty),+ $(,)?) => {{
        let mut router = $router;
        $(
            router = router
                .route(concat!("/", $prefix), get(serve_embedded::<$assets>))
                .route(concat!("/", $prefix, "/"), get(serve_embedded::<$assets>))
                .route(concat!("/", $prefix, "/*path"), get(serve_embedded::<$assets>));
        )+
        router
    }};
}

/// 嵌入式静态资源路由：按应用前缀挂载
pub fn embedded_router() -> Router {
    embed_app_routes!(
        Router::new(),
        "admin" => AdminAssets,
        "fj200c_information" => Fj200cInformationAssets,
        "fj200c_main" => Fj200cMainAssets,
        "fw100" => Fw100Assets,
        "fw150" => Fw150Assets,
        "ftj1c" => Ftj1cAssets,
        "city3d" => City3dAssets,
        "protocol_generator" => ProtocolGeneratorAssets,
    )
}
