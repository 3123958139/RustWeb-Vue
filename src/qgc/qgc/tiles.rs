//! # 地图瓦片代理与磁盘缓存（离线保存 / 加载）
//!
//! 前端 Leaflet 不再直连瓦片源（默认 OpenStreetMap），改走
//! `GET /api/qgc/tiles/{z}/{x}/{y}`：命中磁盘缓存（`tiles/`）直接返回，
//! 未命中则从瓦片源下载并落盘。断网/内网环境下请求直接读缓存，
//! 实现瓦片的**离线保存与加载**；「保存离线地图」= 前端批量请求瓦片端点
//! 触发逐张落盘，「加载」= 地图浏览时自动命中缓存。
//!
//! ## 存储结构
//!
//! ```text
//! tiles/
//! └── {z}/{x}/{y}.png        # 缩放级 / 列 / 行
//! ```
//!
//! 目录首次写入时自动创建（与 csv/ 平级，无需部署预建）；
//! `clear()` 整体删除后重建。瓦片源 URL 模板由 `config-qgc.ini`
//! `[Tiles] Url` 配置（`{z}/{x}/{y}` 占位符），修改后重启生效。

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// 瓦片缓存目录（相对进程工作目录，与 `csv/` 平级）
pub const TILE_CACHE_DIR: &str = "tiles";

/// 缓存文件路径：`tiles/{z}/{x}/{y}.png`
fn cache_path(z: u32, x: u32, y: u32) -> PathBuf {
    PathBuf::from(TILE_CACHE_DIR)
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

/// 瓦片源 URL（`[Tiles] Url` 模板替换占位符，缺省 OpenStreetMap 单子域）
fn source_url(z: u32, x: u32, y: u32) -> String {
    crate::qgc::config::tiles_url()
        .replace("{z}", &z.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
}

/// 瓦片 HTTP 客户端（惰性单例：15s 超时；UA 标识应用名以满足瓦片源政策）
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("RustWeb-Vue/", env!("CARGO_PKG_VERSION"), " (qgc tile proxy)"))
            .build()
            .expect("构建瓦片 HTTP 客户端失败")
    })
}

/// 获取瓦片：命中磁盘缓存直接返回；未命中从瓦片源下载并落盘
///
/// # 参数
/// - `z`：缩放级别
/// - `x`：瓦片列号（Web Mercator）
/// - `y`：瓦片行号（Web Mercator）
///
/// # 返回值
/// `Ok(Vec<u8>)` 为 PNG 图片字节；`Err` 为下载失败原因（缓存未命中且
/// 瓦片源不可达/非 2xx/空响应）。
pub async fn get_tile(z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
    let path = cache_path(z, x, y);

    // 命中缓存：直接返回，无网络请求（离线加载路径）
    if let Ok(bytes) = fs::read(&path) {
        return Ok(bytes);
    }

    // 未命中：从瓦片源下载
    let resp = client()
        .get(source_url(z, x, y))
        .send()
        .await
        .map_err(|e| format!("瓦片源请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("瓦片源返回 {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("读取瓦片响应失败: {e}"))?;
    if bytes.is_empty() {
        return Err("瓦片源返回空内容".to_string());
    }

    // 落盘（目录自动创建）；写盘失败不影响本次响应（下次请求再补）
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, &bytes);

    Ok(bytes.to_vec())
}

/// 缓存统计：已缓存瓦片数量与占用磁盘字节数
///
/// 遍历 `tiles/{z}/{x}/{y}.png` 三级目录（目录不存在时返回 0）。
pub fn stats() -> (usize, u64) {
    let mut count = 0usize;
    let mut bytes = 0u64;
    let Ok(z_levels) = fs::read_dir(TILE_CACHE_DIR) else {
        return (0, 0);
    };
    for level in z_levels.flatten() {
        let Ok(columns) = fs::read_dir(level.path()) else {
            continue;
        };
        for column in columns.flatten() {
            let Ok(rows) = fs::read_dir(column.path()) else {
                continue;
            };
            for row in rows.flatten() {
                let Ok(meta) = row.metadata() else {
                    continue;
                };
                if meta.is_file() {
                    count += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (count, bytes)
}

/// 清空瓦片缓存（删除 `tiles/` 目录并重建）
pub fn clear() -> Result<(), String> {
    if PathBuf::from(TILE_CACHE_DIR).exists() {
        fs::remove_dir_all(TILE_CACHE_DIR).map_err(|e| format!("删除瓦片缓存失败: {e}"))?;
    }
    fs::create_dir_all(TILE_CACHE_DIR).map_err(|e| format!("创建瓦片缓存目录失败: {e}"))
}