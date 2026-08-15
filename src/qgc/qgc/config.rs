//! # qgc 配置文件管理
//!
//! 复用公共 INI 配置封装（`crate::common::config::Config`），
//! 维护本角色独立的全局实例（config-qgc.ini，`[Udp]` / `[Gcs]` 节）。
//!
//! 单例由公共宏 `config_singleton!` 生成（`OnceLock<Config>` 只读，
//! 服务启动时加载一次，运行期不热替换）。

pub use crate::common::config::Config;

crate::config_singleton!(GLOBAL, global, set_global);

/// 读取配置并返回常用值（缺省时提供默认值，未加载时惰性加载）
///
/// # 返回值
/// `(mock, local_port, target_ip, target_port, gcs_sysid, gcs_compid, heartbeat_ms, telemetry_hz)`
pub fn udp_params() -> (bool, u16, String, u16, u8, u8, u64, u16) {
    if global().is_none() {
        if let Ok(cfg) = Config::load(crate::qgc::state::CONFIG_PATH) {
            let _ = set_global(cfg);
        }
    }
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
