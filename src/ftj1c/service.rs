//! # 服务启动/停止编排
//!
//! 读取 config-ftj1c.ini，按 `[Udp] Mock` 开关启动模拟或真实 UDP 工作线程，
//! 管理运行标志与线程句柄。
//!
//! ## 职责
//! - 加载配置文件并初始化全局 `Config` 实例
//! - 启动/停止 UDP 工作线程（模拟模式或真实模式）
//! - 管理线程句柄的生命周期（启动、停止、等待退出）
//! - 提供服务状态查询和 IP 配置获取接口
//!
//! ## 线程安全
//! - 使用 `AtomicBool` 管理运行状态和停止信号，支持多线程无锁读写
//! - 使用公共 `ServiceRuntime`（`crate::common::service`）存储线程句柄
//! - 停止服务时在独立线程中 `join`，避免阻塞 HTTP 请求处理

use crate::common::service::ServiceRuntime;
use crate::ftj1c::config::{self, Config};
use crate::ftj1c::process;
use crate::ftj1c::state::{reset_stop_signal, stop_signal, CONFIG_PATH, SERVICE_RUNNING, quad_frame};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// 公共服务运行时：线程句柄存储 + 停止进行中标志
static RUNTIME: ServiceRuntime = ServiceRuntime::new();

/// 检查服务是否正在运行
///
/// `true` 表示服务已启动，`false` 表示未启动或已停止
pub fn is_running() -> bool {
    SERVICE_RUNNING.load(Ordering::Relaxed)
}

/// 启动通信监控服务
///
/// 1. 检查服务是否已在运行（防止重复启动）
/// 2. 加载 `config-ftj1c.ini` 配置文件并设置全局配置
/// 3. 等待上一次停止流程完成（最多 3 秒，防止竞态条件）
/// 4. 重置停止信号并启动 UDP 工作线程
/// 5. 保存线程句柄并更新运行状态
pub fn start_service(tx: broadcast::Sender<crate::common::ws::EventPayload>) -> Result<(), String> {
    if SERVICE_RUNNING.load(Ordering::Relaxed) {
        return Err("服务已在运行中".to_string());
    }

    let cfg = Config::load(CONFIG_PATH).map_err(|e| format!("加载配置文件失败: {}", e))?;
    let _ = config::set_global(cfg.clone());

    // 确保上一次停止流程已完成，避免与重启竞态
    RUNTIME.wait_stopping(Duration::from_secs(3));

    reset_stop_signal();
    RUNTIME.set_stopping(false);

    let qf = quad_frame();
    let stop = stop_signal();
    let handles = process::start_all(qf, stop, tx);

    for handle in handles {
        RUNTIME.push(handle);
    }

    SERVICE_RUNNING.store(true, Ordering::Relaxed);
    info!("[ftj1c] 通信监控服务已启动");
    Ok(())
}

/// 停止通信监控服务
///
/// 停止操作是异步的，HTTP 请求会立即返回，实际停止在后台线程中完成。
pub fn stop_service() {
    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        return;
    }

    RUNTIME.set_stopping(true);
    stop_signal().store(true, Ordering::Relaxed);

    // 在独立线程中 join，避免阻塞 HTTP 请求处理
    thread::spawn(|| {
        for handle in RUNTIME.drain() {
            let _ = handle.join();
        }
        SERVICE_RUNNING.store(false, Ordering::Relaxed);
        RUNTIME.set_stopping(false);
        info!("[ftj1c] 通信监控服务已停止");
    });
}

/// 获取 IP 配置
///
/// # 返回值
/// `serde_json::Value` 对象，包含 16 组 IP 和端口配置：
/// ```json
/// {
///   "ip1": "192.168.1.1",
///   "port1": 8004,
///   "ip2": "192.168.1.2",
///   "port2": 8005,
///   ...
///   "ip16": "192.168.1.16",
///   "port16": 8006
/// }
/// ```
///
/// # 说明
/// 从 `config-ftj1c.ini` 的 `[IP]` 节读取 `IP1~IP16` 和 `Port1~Port16`。
/// 如果全局配置未加载（前端在服务启动前查询），会自动加载配置文件。
pub fn get_ip_config() -> serde_json::Value {
    // 未加载时惰性加载（前端在服务启动前查询 IP 配置的场景）
    if config::global().is_none() {
        if let Ok(cfg) = Config::load(CONFIG_PATH) {
            let _ = config::set_global(cfg);
        }
    }
    let cfg = config::global();
    let get = |key: &str, default: &str| {
        cfg.map(|c| c.get_or("IP", key, default))
            .unwrap_or_else(|| default.to_string())
    };
    let mut map = serde_json::Map::new();
    for i in 1..=16 {
        map.insert(
            format!("ip{}", i),
            serde_json::Value::String(get(&format!("IP{}", i), "")),
        );
        map.insert(
            format!("port{}", i),
            serde_json::Value::from(
                get(&format!("Port{}", i), "0").parse::<u16>().unwrap_or(0),
            ),
        );
    }
    serde_json::Value::Object(map)
}

/// 验证已保存的配置文件可正常加载
///
/// # 说明
/// `OnceLock` 全局配置在服务启动后不可替换，因此这里仅校验新配置文件的
/// 语法有效性（加载失败立即提示），并提醒运行中的服务需重启后生效。
/// 真正的配置重载在 `start_service` 中完成。
pub fn reload_config() {
    match Config::load(CONFIG_PATH) {
        Ok(_) => warn!("[ftj1c] 配置已保存（重启服务后生效）"),
        Err(e) => warn!("[ftj1c] 配置加载失败: {}", e),
    }
}
