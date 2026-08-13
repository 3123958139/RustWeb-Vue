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

static RUNTIME: ServiceRuntime = ServiceRuntime::new();

pub fn is_running() -> bool {
    state::SERVICE_RUNNING.load(Ordering::Relaxed)
}

pub fn start_service(tx: broadcast::Sender<crate::common::ws::EventPayload>) -> Result<(), String> {
    if is_running() {
        return Err("服务已在运行中".to_string());
    }
    RUNTIME.wait_stopping(Duration::from_secs(3));

    let cfg = Config::load(state::CONFIG_PATH).map_err(|e| format!("加载配置文件失败: {}", e))?;
    config::set_global(cfg);

    GlobalVar::init();
    if let Some(gv) = GlobalVar::global() {
        gv.set("PathCSV", "csv");
    }

    // 复用共享单例（可能已被模拟运行惰性初始化），避免多实例数据不一致
    let shared = state::shared_port_data()
        .cloned()
        .ok_or("共享端口数据初始化失败")?;

    let ports = init_all_from_config(&shared, tx.clone());
    *state::ALL_COM_PORTS
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(ports);

    let proc_stop = start_processing_thread(shared.clone(), tx.clone());
    *state::PROCESSING_STOP
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(proc_stop);

    state::SERVICE_RUNNING.store(true, Ordering::Relaxed);
    info!("fj200c_main 服务已启动");
    Ok(())
}

pub fn stop_service() {
    if !is_running() {
        return;
    }
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

    state::SERVICE_RUNNING.store(false, Ordering::Relaxed);
    RUNTIME.set_stopping(false);
    info!("fj200c_main 服务已停止");
}

pub fn send_command(hex: &str) -> Result<(), String> {
    if !is_running() {
        return Err("服务未运行".to_string());
    }
    let frame = crate::common::utils::parse_hex(hex).ok_or("无效的十六进制指令")?;
    if frame.len() < 16 {
        return Err(format!("指令帧长度不足（当前 {} 字节，至少 16）", frame.len()));
    }
    state::ecu_send_data().store(Arc::new(hex.to_string()));
    Ok(())
}

pub fn toggle_csv_recording(tx: &broadcast::Sender<crate::common::ws::EventPayload>) -> Result<(), String> {
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

pub fn toggle_simulation(tx: &broadcast::Sender<crate::common::ws::EventPayload>) {
    let is_sim = state::SIMULATION_MODE.load(Ordering::Relaxed);
    let new_sim = !is_sim;

    if new_sim {
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

pub fn set_theme(is_dark: bool, tx: &broadcast::Sender<crate::common::ws::EventPayload>) {
    state::THEME_IS_DARK.store(if is_dark { 1 } else { 0 }, Ordering::Relaxed);
    let event = Fj200cMainEvent::ThemeState { is_dark };
    if let Ok(json) = crate::common::ws::serialize(&event) {
        let _ = tx.send(json);
    }
}

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
