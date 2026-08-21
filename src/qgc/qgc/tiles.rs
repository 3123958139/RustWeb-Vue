//! # 地图瓦片代理与磁盘缓存（离线保存 / 加载）
//!
//! 前端 Cesium 不再直连瓦片源（默认高德路网图，`[Tiles] Url` 可换），改走
//! `GET /api/qgc/tiles/{z}/{x}/{y}`：命中磁盘缓存（`tiles/`）直接返回，
//! 未命中则从瓦片源下载并落盘。断网/内网环境下请求直接读缓存，
//! 实现瓦片的**离线保存与加载**；「保存离线地图」= 前端批量请求瓦片端点
//! 触发逐张落盘，「加载」= 地图浏览时自动命中缓存。
//!
//! ## 实现要点
//!
//! - 全程 `tokio::fs` 异步 IO，避免阻塞 tokio worker 线程
//! - **in-flight 去重**：同一瓦片并发请求合并为一个下载任务（Cesium
//!   批量加载 + 离线保存并发时的常见场景），等待者经 `Notify` 唤醒后读缓存
//! - **内容校验**：下载结果校验 PNG/JPEG magic，防止瓦片源返回的 HTML
//!   错误页被永久缓存
//! - **原子落盘**：写临时文件后 rename，进程崩溃不残留半截缓存
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

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::Mutex;
use std::sync::Arc;
use tokio::sync::Notify;

/// 瓦片缓存目录（相对进程工作目录，与 `csv/` 平级）
pub const TILE_CACHE_DIR: &str = "tiles";

/// 缓存文件路径：`tiles/{z}/{x}/{y}.png`
fn cache_path(z: u32, x: u32, y: u32) -> PathBuf {
    PathBuf::from(TILE_CACHE_DIR)
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

/// 进行中的瓦片下载任务（in-flight 去重：同一瓦片只下载一次，等待者等唤醒后读缓存）
static IN_FLIGHT: OnceLock<Mutex<HashMap<(u32, u32, u32), Arc<Notify>>>> = OnceLock::new();

fn in_flight() -> &'static Mutex<HashMap<(u32, u32, u32), Arc<Notify>>> {
    IN_FLIGHT.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 瓦片源 URL（`[Tiles] Url` 模板替换占位符，缺省高德路网图）
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

/// 校验图片 magic：PNG（`\x89PNG\r\n\x1a\n`）或 JPEG（`\xFF\xD8\xFF`）
fn looks_like_image(bytes: &[u8]) -> bool {
    const PNG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() >= PNG.len() && bytes[..PNG.len()] == PNG {
        return true;
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return true;
    }
    false
}

/// 下载瓦片并原子落盘（目录自动创建；写盘失败不影响本次响应，下次请求再补）
async fn download_and_cache(z: u32, x: u32, y: u32, path: &PathBuf) -> Result<Vec<u8>, String> {
    let resp = client()
        .get(source_url(z, x, y))
        .send()
        .await
        .map_err(|e| format!("瓦片源请求失败: {e}（请检查网络或 config-qgc.ini 的 [Tiles] Url 瓦片源配置）"))?;
    if !resp.status().is_success() {
        return Err(format!("瓦片源返回 {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("读取瓦片响应失败: {e}"))?;
    if bytes.is_empty() {
        return Err("瓦片源返回空内容".to_string());
    }
    // 内容校验：非图片内容（如反爬 HTML 错误页）不落盘，避免污染缓存
    if !looks_like_image(&bytes) {
        return Err(format!("瓦片源返回非图片内容（{} 字节，可能是错误页）", bytes.len()));
    }

    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    // 原子落盘：先写临时文件再 rename，避免崩溃留下半截缓存
    let tmp = path.with_extension("tmp");
    if tokio::fs::write(&tmp, &bytes).await.is_ok() {
        let _ = tokio::fs::rename(&tmp, path).await;
    }
    let _ = tokio::fs::remove_file(&tmp).await;

    Ok(bytes.to_vec())
}

/// 获取瓦片：命中磁盘缓存直接返回；未命中从瓦片源下载并落盘
///
/// 同一瓦片的并发请求合并为一次下载（in-flight 去重），等待者经
/// `Notify` 唤醒后重新读缓存。
///
/// # 参数
/// - `z`：缩放级别
/// - `x`：瓦片列号（Web Mercator）
/// - `y`：瓦片行号（Web Mercator）
///
/// # 返回值
/// `Ok(Vec<u8>)` 为图片字节；`Err` 为下载失败原因（缓存未命中且
/// 瓦片源不可达/非 2xx/空响应/非图片内容）。
pub async fn get_tile(z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
    let path = cache_path(z, x, y);

    // 命中缓存：直接返回，无网络请求（离线加载路径）
    if let Ok(bytes) = tokio::fs::read(&path).await {
        return Ok(bytes);
    }

    // 未命中：检查是否已有请求在下载同一瓦片
    // 注意：锁作用域内不做任何 await（std::sync::MutexGuard 非 Send，
    // 跨 await 会导致 future 不满足 axum Handler 的 Send 要求）
    let wait_notify: Option<Arc<Notify>> = {
        let mut map = in_flight().lock().unwrap_or_else(|e| e.into_inner());
        match map.get(&(z, x, y)) {
            // 已有下载者：取出通知位，等其完成后读缓存
            Some(n) => Some(n.clone()),
            // 我是下载者：注册通知位后执行下载
            None => {
                map.insert((z, x, y), Arc::new(Notify::new()));
                None
            }
        }
    };

    if let Some(n) = wait_notify {
        n.notified().await;
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(bytes);
        }
        // 等待者被唤醒但缓存仍不存在（下载者失败）：直接报错，避免并发重试风暴
        return Err(format!("瓦片 {z}/{x}/{y} 下载失败（并发请求等待超时）"));
    }

    let result = download_and_cache(z, x, y, &path).await;

    // 清理 in-flight 登记并唤醒等待者（成功则读缓存，失败则它们自行报错）
    let mut map = in_flight().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(n) = map.remove(&(z, x, y)) {
        drop(map);
        n.notify_waiters();
    }

    result
}

/// 缓存统计：已缓存瓦片数量与占用磁盘字节数
///
/// 遍历 `tiles/{z}/{x}/{y}.png` 三级目录（目录不存在时返回 0）。
/// 在异步上下文中经 `spawn_blocking` 执行，避免阻塞 tokio worker。
pub async fn stats() -> (usize, u64) {
    tokio::task::spawn_blocking(stats_sync)
        .await
        .unwrap_or((0, 0))
}

/// 同步遍历统计（供 `spawn_blocking` 使用）
fn stats_sync() -> (usize, u64) {
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
                // 跳过下载中的临时文件
                let name = row.file_name();
                if name.to_string_lossy().ends_with(".tmp") {
                    continue;
                }
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
///
/// 异步上下文中经 `spawn_blocking` 执行。
pub async fn clear() -> Result<(), String> {
    tokio::task::spawn_blocking(clear_sync)
        .await
        .map_err(|e| format!("清空瓦片缓存任务失败: {e}"))?
}

/// 同步清空实现（供 `spawn_blocking` 使用）
fn clear_sync() -> Result<(), String> {
    if PathBuf::from(TILE_CACHE_DIR).exists() {
        fs::remove_dir_all(TILE_CACHE_DIR).map_err(|e| format!("删除瓦片缓存失败: {e}"))?;
    }
    fs::create_dir_all(TILE_CACHE_DIR).map_err(|e| format!("创建瓦片缓存目录失败: {e}"))
}
