//! 五路串口的初始化、接收线程与处理线程（fj200c_main 模块）
//!
//! 五路设备（ECU/Adam4015/Adam4117/测功机/燃油流量计）以宏
//! `define_com_port!` 生成同类包装结构（`ECUCom` 等），统一流程：
//! 打开串口 → 起接收线程（`abstract_com.rs`）→ 校验/解码帧 → 写入
//! 共享状态 `SharedPortData`（原始帧 `LatestFrame` + 解码结果 `ArcSwap`）
//! → 预序列化后广播 WebSocket。
//!
//! 另含：CSV 录制写行线程（`start_processing_thread`）、ECU 指令周期
//! 下发线程（`init_ecu` 内）、模拟数据发送线程（`start_mock_senders`）。
use crate::common::latest_frame::LatestFrame;
use crate::fj200c_main::abstract_com::*;
use crate::fj200c_main::config;
use crate::fj200c_main::decode::{
    decode_adam4015, decode_adam4117, decode_dyno, decode_ecu, decode_flux, validate_adam4015,
    validate_adam4117, validate_dyno, validate_ecu, validate_flux,
};
use crate::fj200c_main::mock::MockControl;
use crate::fj200c_main::state::{CSV_RECORDING, CSV_WRITER, ECU_SEND_COUNTER};
use crate::fj200c_main::types::*;
use crate::fj200c_main::Fj200cMainEvent;
use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast;
use tracing::error;

/// ECU 串口配置节名（`[COM0]`）
pub const ECU_SECTION: &str = "COM0";
/// Adam4015 串口配置节名（`[COM1]`）
pub const ADAM4015_SECTION: &str = "COM1";
/// Adam4117 串口配置节名（`[COM2]`）
pub const ADAM4117_SECTION: &str = "COM2";
/// 测功机串口配置节名（`[COM3]`）
pub const DYNO_SECTION: &str = "COM3";
/// 燃油流量计串口配置节名（`[COM4]`）
pub const FLUX_SECTION: &str = "COM4";

/// 五路端口的共享状态：原始帧（LatestFrame 无锁读）与解码结果（ArcSwap 无锁读）
///
/// 由 `create_shared_port_data` 创建后经 `state::shared_port_data` 全局持有，
/// 接收线程/模拟线程写，处理线程与 WebSocket handler 读。
pub struct SharedPortData {
    /// ECU 原始帧
    pub ecu_raw: LatestFrame<256>,
    /// Adam4015 原始帧
    pub adam4015_raw: LatestFrame<256>,
    /// Adam4117 原始帧
    pub adam4117_raw: LatestFrame<256>,
    /// 测功机原始帧
    pub dyno_raw: LatestFrame<256>,
    /// 燃油流量计原始帧
    pub flux_raw: LatestFrame<256>,
    /// ECU 解码结果
    pub ecu_decoded: ArcSwap<EcuFields>,
    /// Adam4015 解码结果
    pub adam4015_decoded: ArcSwap<Adam4015Fields>,
    /// Adam4117 解码结果
    pub adam4117_decoded: ArcSwap<Adam4117Fields>,
    /// 测功机解码结果
    pub dyno_decoded: ArcSwap<DynoFields>,
    /// 燃油流量计解码结果
    pub flux_decoded: ArcSwap<FluxFields>,
}

/// 创建共享端口数据实例（全部默认值）
pub fn create_shared_port_data() -> Arc<SharedPortData> {
    Arc::new(SharedPortData {
        ecu_raw: LatestFrame::new(),
        adam4015_raw: LatestFrame::new(),
        adam4117_raw: LatestFrame::new(),
        dyno_raw: LatestFrame::new(),
        flux_raw: LatestFrame::new(),
        ecu_decoded: ArcSwap::new(Arc::new(EcuFields::default())),
        adam4015_decoded: ArcSwap::new(Arc::new(Adam4015Fields::default())),
        adam4117_decoded: ArcSwap::new(Arc::new(Adam4117Fields::default())),
        dyno_decoded: ArcSwap::new(Arc::new(DynoFields::default())),
        flux_decoded: ArcSwap::new(Arc::new(FluxFields::default())),
    })
}

/// 生成单路端口包装结构体（ECUCom / Adam4015Com / ...）
///
/// 每个结构体持有一个 `Result<Arc<AbstractCom>, String>`（串口打开可能失败），
/// 对外提供 `send`（指令下发，打开失败时返回错误）与 `run`（启动接收线程）。
/// `run` 内的解码回调流程：
/// 1. 原始帧写入 `shared.<field_raw>`（LatestFrame）
/// 2. 解码字段写入 `shared.<field_decoded>`（ArcSwap）
/// 3. 组装 `Fj200cMainEvent::PortData` 事件，预序列化后广播
macro_rules! define_com_port {
    ($name:ident, $field_raw:ident, $field_decoded:ident, $validator:expr, $decoder:expr, $payload_type:ty, $variant:ident) => {
        /// 单路端口包装（由宏生成）
        pub struct $name {
            /// 串口打开结果（Err 表示打开失败，接收线程不会启动）
            base: Result<Arc<AbstractCom>, String>,
        }

        impl $name {
            /// 按协议描述构造（内部打开串口）
            pub fn new(
                com_spec: ComSpec,
                stop: Arc<AtomicBool>,
                tx: broadcast::Sender<crate::common::ws::EventPayload>,
            ) -> Arc<Self> {
                Arc::new($name {
                    base: AbstractCom::new(com_spec, stop, tx),
                })
            }

            /// 下发指令（串口打开失败时返回错误）
            pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
                match self.base.as_ref() {
                    Ok(base) => base.send(buf),
                    Err(e) => Err(e.clone().into()),
                }
            }

            /// 启动接收线程（校验 + 解码 + 写共享状态 + 广播）
            pub fn run(
                &self,
                shared: &Arc<SharedPortData>,
                tx: broadcast::Sender<crate::common::ws::EventPayload>,
            ) {
                if let Ok(base) = self.base.as_ref() {
                    let slot = shared.clone();
                    let conn_idx = base.conn_idx();
                    let seq = AtomicU32::new(0);
                    base.start_with($validator, move |frame: &[u8]| {
                        let mut buf = [0u8; 256];
                        let n = frame.len().min(256);
                        buf[..n].copy_from_slice(&frame[..n]);
                        let s = seq.fetch_add(1, Ordering::Relaxed);
                        slot.$field_raw.update(s, &buf);

                        let typed_fields: $payload_type = $decoder(frame);
                        let arc_fields = Arc::new(typed_fields);
                        slot.$field_decoded.store(Arc::clone(&arc_fields));

                        let hex = crate::common::utils::format_hex_compact(frame);

                        // 事件以 Arc<ChannelData> 传递（只克隆指针，避免每帧深拷贝），
                        // 预序列化一次后广播（N 个订阅者只克隆 Arc<str> 指针）
                        let event = Fj200cMainEvent::PortData {
                            connection_index: conn_idx,
                            hex,
                            fields: Arc::new(ChannelData::$variant((*arc_fields).clone())),
                        };
                        if let Ok(json) = crate::common::ws::serialize(&event) {
                            let _ = tx.send(json);
                        }

                        true
                    });
                }
            }
        }
    };
}

define_com_port!(
    ECUCom,
    ecu_raw,
    ecu_decoded,
    validate_ecu,
    decode_ecu,
    EcuFields,
    Ecu
);
define_com_port!(
    Adam4015Com,
    adam4015_raw,
    adam4015_decoded,
    validate_adam4015,
    decode_adam4015,
    Adam4015Fields,
    Adam4015
);
define_com_port!(
    Adam4117Com,
    adam4117_raw,
    adam4117_decoded,
    validate_adam4117,
    decode_adam4117,
    Adam4117Fields,
    Adam4117
);
define_com_port!(
    DynoCom,
    dyno_raw,
    dyno_decoded,
    validate_dyno,
    decode_dyno,
    DynoFields,
    Dyno
);
define_com_port!(
    FluxCom,
    flux_raw,
    flux_decoded,
    validate_flux,
    decode_flux,
    FluxFields,
    Flux
);

/// 五路端口包装的集合（由 `init_all_from_config` 按配置 Count 构建，可含 None）
pub struct AllComPorts {
    /// ECU（COM0）
    pub ecu: Option<Arc<ECUCom>>,
    /// Adam4015（COM1）
    pub adam4015: Option<Arc<Adam4015Com>>,
    /// Adam4117（COM2）
    pub adam4117: Option<Arc<Adam4117Com>>,
    /// 测功机（COM3）
    pub dyno: Option<Arc<DynoCom>>,
    /// 燃油流量计（COM4）
    pub flux: Option<Arc<FluxCom>>,
}

/// 按配置 `[COM] Count` 初始化五路端口（含各自的发送线程）
///
/// 配置未加载或 Count 不足时对应端口为 None（跳过构造，不报错）。
pub fn init_all_from_config(
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> AllComPorts {
    let cfg = match config::global() {
        Some(guard) => match guard.as_ref() {
            Some(c) => c.clone(),
            None => {
                tracing::warn!("配置未加载，跳过所有 COM 构造");
                return AllComPorts {
                    ecu: None,
                    adam4015: None,
                    adam4117: None,
                    dyno: None,
                    flux: None,
                };
            }
        },
        None => {
            tracing::warn!("配置未加载，跳过所有 COM 构造");
            return AllComPorts {
                ecu: None,
                adam4015: None,
                adam4117: None,
                dyno: None,
                flux: None,
            };
        }
    };
    let count: usize = cfg.get_or("COM", "Count", "0").parse().unwrap_or(0);

    let stop = Arc::new(AtomicBool::new(false));

    let ecu = if count > 0 {
        init_ecu(ECU_SECTION, 0, stop.clone(), shared, tx.clone())
    } else {
        tracing::info!("COM Count={}，跳过 {}", count, ECU_SECTION);
        None
    };
    let adam4015 = if count > 1 {
        init_adam4015(ADAM4015_SECTION, 1, stop.clone(), shared, tx.clone())
    } else {
        None
    };
    let adam4117 = if count > 2 {
        init_adam4117(ADAM4117_SECTION, 2, stop.clone(), shared, tx.clone())
    } else {
        None
    };
    let dyno = if count > 3 {
        init_dyno(DYNO_SECTION, 3, stop.clone(), shared, tx.clone())
    } else {
        None
    };
    let flux = if count > 4 {
        init_flux(FLUX_SECTION, 4, stop, shared, tx.clone())
    } else {
        None
    };
    AllComPorts {
        ecu,
        adam4015,
        adam4117,
        dyno,
        flux,
    }
}

/// 启动处理线程：录制开启时每 100ms 读共享解码结果写一行 CSV（64 列）
///
/// 录制标志三态：0=停止、1=刚开启（首帧时间基准）、2=录制中。
/// 返回停止标志（置位后线程退出）。
pub fn start_processing_thread(
    shared: Arc<SharedPortData>,
    _tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    thread::spawn(move || {
        let interval = Duration::from_millis(100);
        let mut csv_last_write = Instant::now();
        while !s.load(Ordering::Relaxed) {
            match CSV_RECORDING.load(Ordering::Relaxed) {
                0 => {
                    thread::sleep(interval);
                    continue;
                }
                1 => {
                    csv_last_write = Instant::now();
                    CSV_RECORDING.store(2, Ordering::Relaxed);
                }
                _ => {
                    if csv_last_write.elapsed() >= interval {
                        let time_elapsed: f32 =
                            (csv_last_write.elapsed().as_millis() as f32) / 1000.0f32;
                        if let Ok(guard) = CSV_WRITER.lock() {
                            if let Some(writer) = guard.as_ref() {
                                let dict = crate::fj200c_main::state::csv_header_dict()
                                    .read()
                                    .unwrap_or_else(|e| e.into_inner());
                                let mut row = Vec::with_capacity(dict.len() + 1);
                                row.push(time_elapsed.to_string());

                                let ecu = shared.ecu_decoded.load();
                                let adam4015 = shared.adam4015_decoded.load();
                                let adam4117 = shared.adam4117_decoded.load();
                                let dyno = shared.dyno_decoded.load();
                                let flux = shared.flux_decoded.load();
                                let values =
                                    csv_row_values(&ecu, &adam4015, &adam4117, &dyno, &flux);
                                row.extend(values);

                                let _ = writer.write_row(row);
                            }
                        }
                    }
                }
            }
            thread::sleep(interval);
        }
    });
    stop
}

/// 初始化 ECU 端口（COM0）：接收线程 + 指令周期下发线程
///
/// 下发线程每 100ms 读取 `state::ecu_send_data`（前端下发的指令帧），
/// 覆写序号字节（frame[3]，0~255 循环）与校验字节（frame[15] = 前 15 字节
/// 累加和低 8 位）后发送。
fn init_ecu(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<ECUCom>> {
    let s = Arc::clone(&stop);
    let com_spec = ComSpec::ecu_protocol(section, idx);
    let com = ECUCom::new(com_spec, stop, tx.clone());

    // 串口打开失败（base 为 Err）时读取线程不会启动，发送线程也没有意义，
    // 跳过以杜绝每 100ms 一次的 "ECU send error" 日志刷屏
    if com.base.is_err() {
        tracing::warn!("ECUCom({}) 串口打开失败，跳过发送线程", section);
        return Some(com);
    }

    com.run(shared, tx);
    tracing::info!("ECUCom({}) 构造完成", section);

    let sender = com.clone();
    thread::spawn(move || {
        // 固定 100ms 周期，deadline 累加补偿睡眠漂移
        let interval = Duration::from_millis(100);
        let mut deadline = Instant::now();
        while !s.load(Ordering::Relaxed) {
            deadline += interval;
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
            let data = crate::fj200c_main::state::ecu_send_data().load();
            let mut frame = match crate::common::utils::parse_hex(&data) {
                Some(f) => f,
                None => continue,
            };
            if frame.len() < 16 {
                continue;
            }
            // 序号 0~255 循环覆写，校验 = 前 15 字节累加和取低 8 位
            if ECU_SEND_COUNTER.load(Ordering::Relaxed) == 255u8 {
                ECU_SEND_COUNTER.store(0, Ordering::Relaxed);
            }
            ECU_SEND_COUNTER.fetch_add(1, Ordering::Relaxed);
            frame[3] = ECU_SEND_COUNTER.load(Ordering::Relaxed);
            frame[15] = (frame[..15].iter().map(|x| *x as u16).sum::<u16>() % 256u16) as u8;
            if let Err(e) = sender.send(&frame) {
                error!("ECU send error: {}", e);
            }
        }
    });

    Some(com)
}

/// 初始化 Adam4015 端口（COM1）：接收线程 + 每秒轮询采集命令（`#010`）
fn init_adam4015(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<Adam4015Com>> {
    let com_spec = ComSpec::adam4015_protocol(section, idx);
    let com = Adam4015Com::new(com_spec, stop.clone(), tx.clone());
    com.run(shared, tx);

    let sender = com.clone();
    thread::spawn(move || {
        let cmd = b"#010\r";
        let interval = Duration::from_secs(1);
        let mut deadline = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            deadline += interval;
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if let Err(e) = sender.send(cmd) {
                tracing::warn!("AdamCom 发送 #010 失败: {:?}", e);
                break;
            }
        }
    });

    tracing::info!("AdamCom({}) 构造完成", section);
    Some(com)
}

/// 初始化 Adam4117 端口（COM2）：接收线程 + 每秒轮询采集命令（`#010`）
fn init_adam4117(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<Adam4117Com>> {
    let com_spec = ComSpec::adam4117_protocol(section, idx);
    let com = Adam4117Com::new(com_spec, stop.clone(), tx.clone());
    com.run(shared, tx);

    let sender = com.clone();
    thread::spawn(move || {
        let cmd = b"#010\r";
        let interval = Duration::from_secs(1);
        let mut deadline = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            deadline += interval;
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if let Err(e) = sender.send(cmd) {
                tracing::warn!("AdamCom 发送 #010 失败: {:?}", e);
                break;
            }
        }
    });

    tracing::info!("AdamCom({}) 构造完成", section);
    Some(com)
}

/// 初始化测功机端口（COM3）：只起接收线程（测功机主动上报，无需下发）
fn init_dyno(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<DynoCom>> {
    let com_spec = ComSpec::dyno_protocol(section, idx);
    let com = DynoCom::new(com_spec, stop, tx.clone());
    com.run(shared, tx);
    tracing::info!("DynoCom({}) 构造完成", section);
    Some(com)
}

/// 初始化燃油流量计端口（COM4）：只起接收线程（设备主动上报，无需下发）
fn init_flux(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<FluxCom>> {
    let com_spec = ComSpec::flux_protocol(section, idx);
    let com = FluxCom::new(com_spec, stop, tx.clone());
    com.run(shared, tx);
    tracing::info!("FluxCom({}) 构造完成", section);
    Some(com)
}

/// 启动五路模拟数据发送线程（`[MOCK_COM0..4]` 配置节驱动）
///
/// 每路按各自 `interval_ms` 周期生成模拟帧，经与真实端口相同的
/// 校验/解码流程写入共享状态并广播；返回停止标志，`stop_mock_senders` 置位后全部退出。
pub fn start_mock_senders(
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));

    // 模拟配置节 → 端口序号映射（MOCK_COM0 = ECU，依此类推）
    let mock_configs: &[(&str, usize)] = &[
        ("MOCK_COM0", 0),
        ("MOCK_COM1", 1),
        ("MOCK_COM2", 2),
        ("MOCK_COM3", 3),
        ("MOCK_COM4", 4),
    ];

    for &(mock_sec, conn_idx) in mock_configs {
        let mock = match MockControl::create(mock_sec) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("模拟发生器 {} 创建失败: {}", mock_sec, e);
                continue;
            }
        };
        let interval = Duration::from_millis(mock.interval_ms());
        let s = stop.clone();
        let shared_clone = shared.clone();
        let tx_clone = tx.clone();
        let tag = mock_sec.to_string();
        tracing::info!("模拟发送器 [{}] 启动, interval={:?}", tag, interval);

        thread::spawn(move || {
            let seq = AtomicU32::new(0);
            while !s.load(Ordering::Relaxed) {
                let frame = mock.generate_frame();

                match conn_idx {
                    0 => {
                        if validate_ecu(&frame) {
                            let fields = decode_ecu(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.ecu_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.ecu_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 0,
                                hex,
                                fields: Arc::new(ChannelData::Ecu(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    1 => {
                        if validate_adam4015(&frame) {
                            let fields = decode_adam4015(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.adam4015_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.adam4015_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 1,
                                hex,
                                fields: Arc::new(ChannelData::Adam4015(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    2 => {
                        if validate_adam4117(&frame) {
                            let fields = decode_adam4117(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.adam4117_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.adam4117_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 2,
                                hex,
                                fields: Arc::new(ChannelData::Adam4117(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    3 => {
                        if validate_dyno(&frame) {
                            let fields = decode_dyno(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.dyno_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.dyno_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 3,
                                hex,
                                fields: Arc::new(ChannelData::Dyno(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    4 => {
                        if validate_flux(&frame) {
                            let fields = decode_flux(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.flux_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.flux_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 4,
                                hex,
                                fields: Arc::new(ChannelData::Flux(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    _ => {}
                }
                thread::sleep(interval);
            }
        });
    }

    stop
}

/// 停止全部模拟发送线程（置位停止标志）
pub fn stop_mock_senders(stop: &AtomicBool) {
    stop.store(true, Ordering::Relaxed);
}
