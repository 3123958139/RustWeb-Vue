//! 服务生命周期与业务操作（fj200c_main 模块）
//!
//! 对外提供：服务启停（`start_service`/`stop_service`）、ECU 指令下发
//! （`send_command`）、CSV 录制开关（`toggle_csv_recording`）、模拟运行开关
//! （`toggle_simulation`）、主题切换（`set_theme`）、试验信息读写。
//!
//! 线程模型：服务启动时打开五路串口并各起接收线程（`com.rs`），另起
//! 处理线程（`com.rs::start_processing_thread`）轮询共享状态、写 CSV 并
//! 广播 WebSocket；停止时统一回收（`stop_all`）。模拟运行可脱离服务
//! 独立启动发送线程（`mock.rs`）。
use crate::common::csv_writer::CsvWriter;
use crate::common::global_var::GlobalVar;
use crate::common::service::ServiceRuntime;
use crate::fj200c_main::com::{
    init_all_from_config, start_mock_senders, start_processing_thread, stop_mock_senders,
};
use crate::fj200c_main::config::{self, Config};
use crate::fj200c_main::state;
use crate::fj200c_main::types::ExperimentInfo;
use crate::fj200c_main::Fj200cMainEvent;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{error, info};

/// 服务运行时状态机（公共组件，管理 启动中/停止中 状态）
static RUNTIME: ServiceRuntime = ServiceRuntime::new();

/// 服务是否运行中（等价于 `state::SERVICE_RUNNING`）
pub fn is_running() -> bool {
    state::SERVICE_RUNNING.load(Ordering::Relaxed)
}

/// 启动服务：加载配置 → 打开五路串口 → 启动处理线程
///
/// 失败返回中文错误信息；重复启动直接报"服务已在运行中"。
pub fn start_service(tx: broadcast::Sender<crate::common::ws::EventPayload>) -> Result<(), String> {
    if is_running() {
        return Err("服务已在运行中".to_string());
    }
    // 排他运行（公共组件）：有且只有当前角色保持线程与资源，停止其他角色的服务
    crate::common::service::stop_all_services_except(Some("fj200c_main"));
    RUNTIME.wait_stopping(Duration::from_secs(3));

    let cfg = Config::load(state::CONFIG_PATH).map_err(|e| format!("加载配置文件失败: {}", e))?;
    config::set_global(cfg);

    // 初始化全局变量容器，CSV 目录固定为 csv/
    GlobalVar::init();
    if let Some(gv) = GlobalVar::global() {
        gv.set("PathCSV", "csv");
    }

    // 复用共享单例（可能已被模拟运行惰性初始化），避免多实例数据不一致
    let shared = state::shared_port_data()
        .cloned()
        .ok_or("共享端口数据初始化失败")?;

    // 五路串口（含模拟源）初始化
    let ports = init_all_from_config(&shared, tx.clone());
    *state::ALL_COM_PORTS
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(ports);

    // 处理线程：轮询共享状态 → CSV 录制 / WebSocket 广播
    let proc_stop = start_processing_thread(shared.clone(), tx.clone());
    *state::PROCESSING_STOP
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(proc_stop);

    state::SERVICE_RUNNING.store(true, Ordering::Relaxed);
    info!("fj200c_main 服务已启动");
    Ok(())
}

/// 停止服务（幂等，见 `stop_all`）
pub fn stop_service() {
    stop_all();
}

/// 停止服务与模拟运行的全部线程资源（幂等，未运行也安全）
///
/// 除 `stop_service` 的常规停止外，还额外处理：
/// - 服务未运行时单独启动的模拟发送线程（`/simulation/toggle`）
/// - CSV 录制：复位标志并 flush 关闭文件，避免残留打开句柄
///
/// 登出 / 切换角色时由 `crate::common::service::stop_all_services` 统一调用。
pub fn stop_all() {
    RUNTIME.set_stopping(true);

    if let Ok(mut guard) = state::ALL_COM_PORTS.lock() {
        guard.take();
    }
    if let Ok(mut guard) = state::PROCESSING_STOP.lock() {
        if let Some(stop) = guard.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }
    let mut mock_stopped = false;
    if let Ok(mut guard) = state::MOCK_SENDERS_STOP.lock() {
        if let Some(stop) = guard.take() {
            stop_mock_senders(&stop);
            mock_stopped = true;
        }
    }

    // 模拟发送线程被停止时同步复位模拟状态并广播，避免前端"模拟运行中"徽章与真实状态不一致
    if mock_stopped {
        state::SIMULATION_MODE.store(false, Ordering::Relaxed);
        let event = Fj200cMainEvent::SimulationState { simulating: false };
        if let Ok(json) = crate::common::ws::serialize(&event) {
            let _ = crate::fj200c_main::fj200c_main_tx().send(json);
        }
    }

    // 停止录制并 flush 关闭 CSV 文件（服务未运行但录制残留时同样清理）
    state::CSV_RECORDING.store(0, Ordering::Relaxed);
    if let Ok(mut guard) = state::CSV_WRITER.lock() {
        if let Some(writer) = guard.take() {
            let _ = writer.flush();
        }
    }

    state::SERVICE_RUNNING.store(false, Ordering::Relaxed);
    RUNTIME.set_stopping(false);
    info!("fj200c_main 服务已停止");
}

/// 下发 ECU 指令（十六进制帧）
///
/// 校验长度 ≥ 16 字节后存入 `state::ecu_send_data`，由发送线程
/// （`com.rs::start_processing_thread`）周期下发到 ECU 串口。
pub fn send_command(hex: &str) -> Result<(), String> {
    if !is_running() {
        return Err("服务未运行".to_string());
    }
    let frame = crate::common::utils::parse_hex(hex).ok_or("无效的十六进制指令")?;
    if frame.len() < 16 {
        return Err(format!(
            "指令帧长度不足（当前 {} 字节，至少 16）",
            frame.len()
        ));
    }
    state::ecu_send_data().store(Arc::new(hex.to_string()));
    Ok(())
}

/// 切换 CSV 录制（幂等）：开始写试验信息文件 + 数据文件，或停止并 flush
pub fn toggle_csv_recording(
    tx: &broadcast::Sender<crate::common::ws::EventPayload>,
) -> Result<(), String> {
    let is_recording = state::CSV_RECORDING.load(Ordering::Relaxed);
    match is_recording {
        0 => {
            // 写行循环位于服务处理线程内，服务未运行时录制只会产生空文件
            if !is_running() {
                return Err("服务未运行，无法开始录制".to_string());
            }
            let now = chrono::Local::now().format("%Y%m%d%H%M%S");
            let file_name = format!("recording_{}_information.csv", now);
            match CsvWriter::create("csv", &file_name, vec!["字段".into(), "值".into()]) {
                Ok(writer) => {
                    let info = get_experiment_info();
                    let _ = writer.write_row(vec!["发动机编号".into(), info.engine_no]);
                    let _ = writer.write_row(vec!["燃气发生器编号".into(), info.gas_generator_no]);
                    let _ = writer.write_row(vec!["电控器编号".into(), info.controller_no]);
                    let _ = writer.write_row(vec!["转速传感器编号".into(), info.speed_sensor_no]);
                    let _ =
                        writer.write_row(vec!["滑油温压一体传感器编号".into(), info.oil_sensor_no]);
                    let _ = writer.write_row(vec!["试验项目".into(), info.test_item]);
                    let _ = writer.write_row(vec!["试验时间".into(), info.test_time]);
                    let _ = writer.flush();
                }
                Err(e) => {
                    return Err(format!("创建试验信息文件失败: {}", e));
                }
            }
            let mut headers: Vec<String> = vec!["时间戳".to_string()];
            headers.extend(
                state::csv_header_dict()
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .names()
                    .iter()
                    .map(|s| s.to_string()),
            );
            let filename = format!("recording_{}.csv", now);
            match CsvWriter::create("csv", &filename, headers) {
                Ok(writer) => {
                    *state::CSV_WRITER.lock().unwrap_or_else(|e| e.into_inner()) = Some(writer);
                    state::CSV_RECORDING.store(1, Ordering::Relaxed);
                    info!("数据记录已开始: {}", filename);
                }
                Err(e) => {
                    return Err(format!("创建 CSV 文件失败: {}", e));
                }
            }
        }
        _ => {
            state::CSV_RECORDING.store(0, Ordering::Relaxed);
            if let Ok(mut guard) = state::CSV_WRITER.lock() {
                if let Some(writer) = guard.take() {
                    let _ = writer.flush();
                }
            }
            info!("数据记录已停止");
        }
    }
    let recording = state::CSV_RECORDING.load(Ordering::Relaxed) != 0;
    let event = Fj200cMainEvent::CsvRecordingState { recording };
    if let Ok(json) = crate::common::ws::serialize(&event) {
        let _ = tx.send(json);
    }
    Ok(())
}

/// 切换模拟运行：启动/停止五路模拟数据发送线程
///
/// 模拟线程不依赖服务启动（`shared_port_data` 惰性创建），故服务未运行
/// 时也可单独演示数据流；启动前同样先停止其他角色服务（排他）。
pub fn toggle_simulation(tx: &broadcast::Sender<crate::common::ws::EventPayload>) {
    let is_sim = state::SIMULATION_MODE.load(Ordering::Relaxed);
    let new_sim = !is_sim;

    if new_sim {
        // 排他运行（公共组件）：启动模拟发送线程前停止其他角色的服务线程与资源
        crate::common::service::stop_all_services_except(Some("fj200c_main"));
        // shared_port_data 惰性创建，未启动服务时模拟运行也能正常推送数据
        let mut guard = state::MOCK_SENDERS_STOP
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            let shared = match state::shared_port_data().cloned() {
                Some(shared) => shared,
                None => {
                    error!("共享端口数据初始化失败");
                    return;
                }
            };
            let stop = start_mock_senders(&shared, tx.clone());
            *guard = Some(stop);
        }
    } else {
        if let Some(stop) = state::MOCK_SENDERS_STOP
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            stop_mock_senders(&stop);
        }
    }
    state::SIMULATION_MODE.store(new_sim, Ordering::Relaxed);
    info!("模拟运行状态: {}", if new_sim { "启动" } else { "停止" });
    let event = Fj200cMainEvent::SimulationState {
        simulating: new_sim,
    };
    if let Ok(json) = crate::common::ws::serialize(&event) {
        let _ = tx.send(json);
    }
}

/// 切换界面主题（深浅色），状态广播给所有前端
pub fn set_theme(is_dark: bool, tx: &broadcast::Sender<crate::common::ws::EventPayload>) {
    state::THEME_IS_DARK.store(if is_dark { 1 } else { 0 }, Ordering::Relaxed);
    let event = Fj200cMainEvent::ThemeState { is_dark };
    if let Ok(json) = crate::common::ws::serialize(&event) {
        let _ = tx.send(json);
    }
}

/// 读取试验信息（从全局变量容器，未初始化时返回默认空值）
pub fn get_experiment_info() -> ExperimentInfo {
    let gv = match GlobalVar::global() {
        Some(g) => g,
        None => return ExperimentInfo::default(),
    };
    ExperimentInfo {
        engine_no: gv.get_or("engine_no", ""),
        gas_generator_no: gv.get_or("gas_generator_no", ""),
        controller_no: gv.get_or("controller_no", ""),
        speed_sensor_no: gv.get_or("speed_sensor_no", ""),
        oil_sensor_no: gv.get_or("oil_sensor_no", ""),
        test_item: gv.get_or("test_item", ""),
        test_time: gv.get_or("test_time", ""),
    }
}

/// 保存试验信息（写入全局变量容器，未初始化时先初始化）
pub fn save_experiment_info(info: &ExperimentInfo) -> Result<(), String> {
    let gv = match GlobalVar::global() {
        Some(g) => g,
        _ => {
            GlobalVar::init();
            GlobalVar::global().ok_or("GlobalVar 未初始化")?
        }
    };
    gv.set("engine_no", &info.engine_no);
    gv.set("gas_generator_no", &info.gas_generator_no);
    gv.set("controller_no", &info.controller_no);
    gv.set("speed_sensor_no", &info.speed_sensor_no);
    gv.set("oil_sensor_no", &info.oil_sensor_no);
    gv.set("test_item", &info.test_item);
    gv.set("test_time", &info.test_time);
    info!("试验信息已保存: {:?}", info);
    Ok(())
}
