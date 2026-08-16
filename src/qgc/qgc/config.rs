//! # qgc 配置文件管理
//!
//! 复用公共 INI 配置封装（`crate::common::config::Config`），
//! 维护本角色独立的全局实例（config-qgc.ini，`[Udp]` / `[Gcs]` / `[Tiles]` 节）。
//!
//! 单例由公共宏 `config_singleton!` 生成（`OnceLock<Config>` 只读，
//! 服务启动时加载一次，运行期不热替换）。

pub use crate::common::config::Config;

crate::config_singleton!(GLOBAL, global, set_global);

/// 确保全局配置已加载（服务启动 / handler 首次读取时惰性加载）
fn ensure_loaded() {
    if global().is_none() {
        if let Ok(cfg) = Config::load(crate::qgc::state::CONFIG_PATH) {
            let _ = set_global(cfg);
        }
    }
}

/// 读取配置并返回常用值（缺省时提供默认值，未加载时惰性加载）
///
/// # 返回值
/// `(mock, local_port, target_ip, target_port, gcs_sysid, gcs_compid, heartbeat_ms, telemetry_hz)`
pub fn udp_params() -> (bool, u16, String, u16, u8, u8, u64, u16) {
    ensure_loaded();
    let cfg = global();
    let get = |key: &str, default: &str| {
        cfg.map(|c| c.get_or("Udp", key, default))
            .unwrap_or_else(|| default.to_string())
    };
    let get_gcs = |key: &str, default: &str| {
        cfg.map(|c| c.get_or("Gcs", key, default))
            .unwrap_or_else(|| default.to_string())
    };
    (
        get("Mock", "true") == "true",
        get("LocalPort", "14550").parse().unwrap_or(14550),
        get("TargetIp", "127.0.0.1"),
        get("TargetPort", "14550").parse().unwrap_or(14550),
        get_gcs("SysId", "255").parse().unwrap_or(255),
        get_gcs("CompId", "190").parse().unwrap_or(190),
        get_gcs("HeartbeatMs", "1000").parse().unwrap_or(1000),
        get_gcs("TelemetryHz", "10").parse().unwrap_or(10),
    )
}

/// 读取地图瓦片源 URL 模板（`[Tiles] Url`，支持 `{z}/{x}/{y}` 占位符）
///
/// # 返回值
/// 瓦片源 URL 模板字符串，缺省高德路网图地址（国内可直连；OpenStreetMap
/// 在国内网络环境不可达，需内网部署时可在 `config-qgc.ini` 换成内网源）。
/// 代理下载并缓存到磁盘 `tiles/` 目录，离线/内网环境下直接从缓存加载。
pub fn tiles_url() -> String {
    ensure_loaded();
    global()
        .map(|c| {
            c.get_or(
                "Tiles",
                "Url",
                "https://webrd01.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=8&x={x}&y={y}&z={z}",
            )
        })
        .unwrap_or_else(|| "https://webrd01.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=8&x={x}&y={y}&z={z}".to_string())
}