//! # 服务启动/停止编排
//!
//! 读取 config-qgc.ini，按 `[Udp] Mock` 开关启动模拟飞控线程，
//! 管理运行标志与线程句柄。
//!
//! ## 职责
//! - 加载配置文件并初始化全局 `Config` 实例
//! - 启动/停止工作线程（接收 / 发送 / 模拟飞控）
//! - 管理线程句柄的生命周期（启动、停止、等待退出）
//! - 提供服务状态查询
//!
//! ## 线程安全
//! - 使用 `AtomicBool` 管理运行状态和停止信号，支持多线程无锁读写
//! - 使用公共 `ServiceRuntime`（`crate::common::service`）存储线程句柄
//! - 停止服务时在独立线程中 `join`，避免阻塞 HTTP 请求处理

use crate::common::service::ServiceRuntime;
use crate::qgc::config::{self, Config};
use crate::qgc::process;
use crate::qgc::state::{reset_stop_signal, stop_signal, CONFIG_PATH, SERVICE_RUNNING};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// 公共服务运行时：线程句柄存储 + 停止进行中标志
static RUNTIME: ServiceRuntime = ServiceRuntime::new();

/// 检查服务是否正在运行
pub fn is_running() -> bool {
    SERVICE_RUNNING.load(Ordering::Relaxed)
}

/// 启动飞控通信服务
///
/// 1. 检查服务是否已在运行（防止重复启动）
/// 2. 加载 `config-qgc.ini` 配置文件并设置全局配置
/// 3. 等待上一次停止流程完成（最多 3 秒，防止竞态条件）
/// 4. 重置停止信号与状态，启动工作线程
/// 5. 保存线程句柄并更新运行状态
pub fn start_service(tx: broadcast::Sender<crate::common::ws::EventPayload>) -> Result<(), String> {
    if SERVICE_RUNNING.load(Ordering::Relaxed) {
        return Err("服务已在运行中".to_string());
    }
    // 排他运行（公共组件）：有且只有当前角色保持线程与资源，停止其他角色的服务
    crate::common::service::stop_all_services_except(Some("qgc"));

    let cfg = Config::load(CONFIG_PATH).map_err(|e| format!("加载配置文件失败: {}", e))?;
    let _ = config::set_global(cfg.clone());

    // 确保上一次停止流程已完成，避免与重启竞态
    RUNTIME.wait_stopping(Duration::from_secs(3));

    reset_stop_signal();
    RUNTIME.set_stopping(false);

    let stop = stop_signal();
    let handles = process::start_all(stop, tx)?;

    for handle in handles {
        RUNTIME.push(handle);
    }

    SERVICE_RUNNING.store(true, Ordering::Relaxed);
    info!("[qgc] 飞控通信服务已启动");
    Ok(())
}

/// 停止飞控通信服务
///
/// 停止操作是异步的，HTTP 请求会立即返回，实际停止在后台线程中完成。
pub fn stop_service() {
    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        return;
    }
    crate::common::service::stop_in_background(
        &RUNTIME,
        &SERVICE_RUNNING,
        || {
            stop_signal().store(true, Ordering::Relaxed);
            crate::qgc::state::clear_outbound();
        },
        "[qgc] 飞控通信服务已停止",
    );
}

/// 验证已保存的配置文件可正常加载
///
/// `OnceLock` 全局配置在服务启动后不可替换，因此这里仅校验新配置文件的
/// 语法有效性（加载失败立即提示），并提醒运行中的服务需重启后生效。
pub fn reload_config() {
    match Config::load(CONFIG_PATH) {
        Ok(_) => warn!("[qgc] 配置已保存（重启服务后生效）"),
        Err(e) => warn!("[qgc] 配置加载失败: {}", e),
    }
}
