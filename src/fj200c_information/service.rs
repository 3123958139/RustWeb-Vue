//! # 服务启动/停止编排
//!
//! 读取 config-fj200c_information.ini，按连接配置启动会话线程，管理运行标志。
//! 从 fj200c_information.informatization 的 backend/service.rs 移植。
//!
//! ## 关键语法
//!
//! - **`ServiceRuntime`**：公共线程句柄 / 停止标志管理
//!   （`crate::common::service`），停止服务时依次 join，确保会话线程干净退出。
//! - **`broadcast::Sender` 的同步 send**：可以在 std 线程中调用，
//!   WebSocket 任务（tokio）异步接收。
//! - **`SERVICE_RUNNING: AtomicBool`**：供 HTTP 层查询运行状态。

use crate::common::service::ServiceRuntime;
use crate::common::ws::EventPayload;
use crate::fj200c_information::com::ComControl;
use crate::fj200c_information::config::{self, Config};
use crate::fj200c_information::mock::{MockControl, STOP_SIGNAL};
use crate::fj200c_information::mock_feeder::start_mock_feeder;
use crate::fj200c_information::session::{init_command_channel, run_one_connection};
use crate::fj200c_information::state::SERVICE_RUNNING;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{error, info};

/// 公共服务运行时：线程句柄存储 + 停止进行中标志
static RUNTIME: ServiceRuntime = ServiceRuntime::new();

/// 最大支持的连接数（Connection0 ~ Connection7）
const MAX_CONNECTIONS: usize = 8;

/// 查询发动机监控服务是否正在运行
///
/// 使用 `AtomicBool` 的 `Relaxed` 内存序进行无锁读取，
/// 适合仅需要最终一致性的状态查询场景。
pub fn is_running() -> bool {
    SERVICE_RUNNING.load(Ordering::Relaxed)
}

/// 启动服务：加载配置，为每个启用的连接启动会话线程
pub fn start_service(tx: broadcast::Sender<EventPayload>) -> Result<(), String> {
    if SERVICE_RUNNING.load(Ordering::Relaxed) {
        return Err("服务已在运行中".to_string());
    }
    let cfg = Config::load(crate::fj200c_information::state::CONFIG_PATH)
        .map_err(|e| format!("加载配置文件失败: {}", e))?;
    let _ = config::set_global(cfg.clone());

    // 确保上一次停止流程已完成，避免与重启竞态
    RUNTIME.wait_stopping(Duration::from_secs(3));

    init_command_channel();
    STOP_SIGNAL.store(false, Ordering::Relaxed);
    RUNTIME.set_stopping(false);

    let mock_enabled = cfg
        .get_or("Mock", "InProcess", "true")
        .eq_ignore_ascii_case("true");
    let mut started = 0usize;

    for i in 0..MAX_CONNECTIONS {
        let section = format!("Connection{}", i);
        let enabled = cfg
            .get_or(&section, "Enabled", "false")
            .eq_ignore_ascii_case("true");
        if !enabled {
            continue;
        }

        let tx_clone = tx.clone();
        let port_name = cfg.get_or(&section, "ComPort", &format!("COM{}", i + 1));
        let baud_rate = cfg
            .get_or(&section, "BaudRate", "115200")
            .parse::<u32>()
            .unwrap_or(115200);
        let data_bits = cfg.get_or(&section, "DataBits", "8").parse().unwrap_or(8);
        let stop_bits = cfg.get_or(&section, "StopBits", "1").parse().unwrap_or(1);
        let parity = cfg.get_or(&section, "Parity", "0").parse().unwrap_or(0);
        let flow_control = cfg
            .get_or(&section, "FlowControl", "false")
            .eq_ignore_ascii_case("true");
        let feeder_enabled = cfg
            .get_or("Mock", "FeederMode", "false")
            .eq_ignore_ascii_case("true");
        let cfg_clone = cfg.clone();

        let handle = thread::spawn(move || {
            if mock_enabled {
                info!("连接 {}: 使用进程内模拟数据源", i);
                let control: Arc<dyn crate::fj200c_information::IoControl> = Arc::new(MockControl::new());
                run_one_connection(i, control, tx_clone, &cfg_clone);
            } else {
                match ComControl::new(
                    &port_name,
                    baud_rate,
                    data_bits,
                    stop_bits,
                    parity,
                    flow_control,
                    &section,
                ) {
                    Ok(com) => {
                        info!("连接 {}: 串口 {} 已打开 ({} baud)", i, port_name, baud_rate);
                        let control: Arc<dyn crate::fj200c_information::IoControl> = Arc::new(com);
                        if feeder_enabled {
                            // 虚拟串口对模式：外部 feeder 写入模拟数据
                            start_mock_feeder(Arc::clone(&control));
                        }
                        run_one_connection(i, control, tx_clone, &cfg_clone);
                    }
                    Err(e) => {
                        error!("连接 {}: 打开串口 {} 失败: {}", i, port_name, e);
                    }
                }
            }
        });

        RUNTIME.push(handle);
        started += 1;
    }

    if started == 0 {
        STOP_SIGNAL.store(true, Ordering::Relaxed);
        return Err(
            "没有启用的连接（请检查 config-fj200c_information.ini 的 ConnectionN.Enabled）".to_string(),
        );
    }

    SERVICE_RUNNING.store(true, Ordering::Relaxed);
    info!("发动机监控服务已启动，共 {} 个连接", started);
    Ok(())
}

/// 停止服务：设置停止信号并等待所有会话线程退出
pub fn stop_service() {
    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        return;
    }

    RUNTIME.set_stopping(true);
    STOP_SIGNAL.store(true, Ordering::Relaxed);

    // 在独立线程中 join，避免阻塞 HTTP 请求处理
    thread::spawn(|| {
        for handle in RUNTIME.drain() {
            let _ = handle.join();
        }
        SERVICE_RUNNING.store(false, Ordering::Relaxed);
        RUNTIME.set_stopping(false);
        info!("发动机监控服务已停止");
    });
}

/// 发送十六进制命令到所有会话线程
pub fn send_command(hex: &str) -> Result<(), String> {
    if !is_running() {
        return Err("服务未运行".to_string());
    }

    let bytes = crate::fj200c_information::mock::parse_command_hex(hex)
        .ok_or_else(|| format!("无效的十六进制命令: {}", hex))?;
    if bytes.is_empty() {
        return Err("命令不能为空".to_string());
    }

    if let Some(cmd_tx) = crate::fj200c_information::session::COMMAND_TX.get() {
        if let Some(sender) = cmd_tx.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            sender
                .send(bytes)
                .map_err(|_| "命令通道已关闭（服务未运行？）".to_string())?;
        }
    }
    Ok(())
}
