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

/// admin 管理后台
#[derive(RustEmbed)]
#[folder = "frontend/admin/dist/"]
struct AdminAssets;

/// fj200c_information 发动机监控
#[derive(RustEmbed)]
#[folder = "frontend/fj200c_information/dist/"]
struct Fj200cInformationAssets;

/// fj200c_main 发动机测控（ECU/ADAM/DYNO 三路串口）
#[derive(RustEmbed)]
#[folder = "frontend/fj200c_main/dist/"]
struct Fj200cMainAssets;

/// fw100 设备台账
#[derive(RustEmbed)]
#[folder = "frontend/fw100/dist/"]
struct Fw100Assets;

/// fw150 设备台账
#[derive(RustEmbed)]
#[folder = "frontend/fw150/dist/"]
struct Fw150Assets;

/// ftj1c UDP 通信监控
#[derive(RustEmbed)]
#[folder = "frontend/ftj1c/dist/"]
struct Ftj1cAssets;

/// city3d 城市 3D 展示
#[derive(RustEmbed)]
#[folder = "frontend/city3d/dist/"]
struct City3dAssets;

/// protocol_generator 通信协议生成
#[derive(RustEmbed)]
#[folder = "frontend/protocol_generator/dist/"]
struct ProtocolGeneratorAssets;

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

/// 嵌入式静态资源路由：按应用前缀挂载
///
/// 每个应用注册三条路由（axum 0.7 通配符语法 `/*path`）：
/// - `/admin`（精确，无斜杠）→ `Option<Path>` 为 `None`，返回 index.html
/// - `/admin/`（精确，带尾斜杠）→ 同上；缺少它时 matchit 会对 `/admin/`
///   触发 ExtraTrailingSlash 检查返回 404（已有实测验证）
/// - `/admin/*path`（通配，含任意深层路径）→ 资源命中或 SPA 回退
pub fn embedded_router() -> Router {
    Router::new()
        .route("/admin", get(serve_embedded::<AdminAssets>))
        .route("/admin/", get(serve_embedded::<AdminAssets>))
        .route("/admin/*path", get(serve_embedded::<AdminAssets>))
        .route("/fj200c_information", get(serve_embedded::<Fj200cInformationAssets>))
        .route("/fj200c_information/", get(serve_embedded::<Fj200cInformationAssets>))
        .route("/fj200c_information/*path", get(serve_embedded::<Fj200cInformationAssets>))
        .route("/fj200c_main", get(serve_embedded::<Fj200cMainAssets>))
        .route("/fj200c_main/", get(serve_embedded::<Fj200cMainAssets>))
        .route("/fj200c_main/*path", get(serve_embedded::<Fj200cMainAssets>))
        .route("/fw100", get(serve_embedded::<Fw100Assets>))
        .route("/fw100/", get(serve_embedded::<Fw100Assets>))
        .route("/fw100/*path", get(serve_embedded::<Fw100Assets>))
        .route("/fw150", get(serve_embedded::<Fw150Assets>))
        .route("/fw150/", get(serve_embedded::<Fw150Assets>))
        .route("/fw150/*path", get(serve_embedded::<Fw150Assets>))
        .route("/ftj1c", get(serve_embedded::<Ftj1cAssets>))
        .route("/ftj1c/", get(serve_embedded::<Ftj1cAssets>))
        .route("/ftj1c/*path", get(serve_embedded::<Ftj1cAssets>))
        .route("/city3d", get(serve_embedded::<City3dAssets>))
        .route("/city3d/", get(serve_embedded::<City3dAssets>))
        .route("/city3d/*path", get(serve_embedded::<City3dAssets>))
        .route("/protocol_generator", get(serve_embedded::<ProtocolGeneratorAssets>))
        .route("/protocol_generator/", get(serve_embedded::<ProtocolGeneratorAssets>))
        .route("/protocol_generator/*path", get(serve_embedded::<ProtocolGeneratorAssets>))
}
