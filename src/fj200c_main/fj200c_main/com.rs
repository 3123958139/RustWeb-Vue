use crate::common::latest_frame::LatestFrame;
use crate::fj200c_main::abstract_com::*;
use crate::fj200c_main::config;
use crate::fj200c_main::decode::{decode_adam, decode_dyno, decode_ecu, validate_adam, validate_dyno, validate_ecu};
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

pub const ECU_SECTION: &str = "COM0";
pub const ADAM_SECTION: &str = "COM1";
pub const DYNO_SECTION: &str = "COM2";

pub struct SharedPortData {
    pub ecu_raw: LatestFrame<256>,
    pub adam_raw: LatestFrame<256>,
    pub dyno_raw: LatestFrame<256>,
    pub ecu_decoded: ArcSwap<EcuFields>,
    pub adam_decoded: ArcSwap<AdamFields>,
    pub dyno_decoded: ArcSwap<DynoFields>,
}

pub fn create_shared_port_data() -> Arc<SharedPortData> {
    Arc::new(SharedPortData {
        ecu_raw: LatestFrame::new(),
        adam_raw: LatestFrame::new(),
        dyno_raw: LatestFrame::new(),
        ecu_decoded: ArcSwap::new(Arc::new(EcuFields::default())),
        adam_decoded: ArcSwap::new(Arc::new(AdamFields::default())),
        dyno_decoded: ArcSwap::new(Arc::new(DynoFields::default())),
    })
}

macro_rules! define_com_port {
    ($name:ident, $field_raw:ident, $field_decoded:ident, $validator:expr, $decoder:expr, $payload_type:ty, $variant:ident) => {
        pub struct $name {
            base: Result<Arc<AbstractCom>, String>,
        }

        impl $name {
            pub fn new(
                com_spec: ComSpec,
                stop: Arc<AtomicBool>,
                tx: broadcast::Sender<crate::common::ws::EventPayload>,
            ) -> Arc<Self> {
                Arc::new($name {
                    base: AbstractCom::new(com_spec, stop, tx),
                })
            }

            pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
                match self.base.as_ref() {
                    Ok(base) => base.send(buf),
                    Err(e) => Err(e.clone().into()),
                }
            }

            pub fn run(&self, shared: &Arc<SharedPortData>, tx: broadcast::Sender<crate::common::ws::EventPayload>) {
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

define_com_port!(ECUCom, ecu_raw, ecu_decoded, validate_ecu, decode_ecu, EcuFields, Ecu);
define_com_port!(AdamCom, adam_raw, adam_decoded, validate_adam, decode_adam, AdamFields, Adam);
define_com_port!(DynoCom, dyno_raw, dyno_decoded, validate_dyno, decode_dyno, DynoFields, Dyno);

pub struct AllComPorts {
    pub ecu: Option<Arc<ECUCom>>,
    pub adam: Option<Arc<AdamCom>>,
    pub dyno: Option<Arc<DynoCom>>,
}

pub fn init_all_from_config(
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> AllComPorts {
    let cfg = match config::global() {
        Some(c) => c,
        None => {
            tracing::warn!("配置未加载，跳过所有 COM 构造");
            return AllComPorts {
                ecu: None,
                adam: None,
                dyno: None,
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
    let adam = if count > 1 {
        init_adam(ADAM_SECTION, 1, stop.clone(), shared, tx.clone())
    } else {
        None
    };
    let dyno = if count > 2 {
        init_dyno(DYNO_SECTION, 2, stop, shared, tx.clone())
    } else {
        None
    };

    AllComPorts { ecu, adam, dyno }
}

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
                                let dict =
                                    crate::fj200c_main::state::csv_header_dict().read().unwrap_or_else(|e| e.into_inner());
                                let mut row = Vec::with_capacity(dict.len() + 1);
                                row.push(time_elapsed.to_string());

                                let ecu = shared.ecu_decoded.load();
                                let adam = shared.adam_decoded.load();
                                let dyno = shared.dyno_decoded.load();
                                let values = csv_row_values(&ecu, &adam, &dyno);
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
    com.run(shared, tx);
    tracing::info!("ECUCom({}) 构造完成", section);

    let sender = com.clone();
    thread::spawn(move || {
        while !s.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
            let data = crate::fj200c_main::state::ecu_send_data().load();
            let mut frame = match crate::common::utils::parse_hex(&data) {
                Some(f) => f,
                None => continue,
            };
            if frame.len() < 16 {
                continue;
            }
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

fn init_adam(
    section: &str,
    idx: usize,
    stop: Arc<AtomicBool>,
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Option<Arc<AdamCom>> {
    let com_spec = ComSpec::adam_protocol(section, idx);
    let com = AdamCom::new(com_spec, stop.clone(), tx.clone());
    com.run(shared, tx);

    let sender = com.clone();
    thread::spawn(move || {
        let cmd = b"#010\r";
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(1));
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

pub fn start_mock_senders(
    shared: &Arc<SharedPortData>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));

    let mock_configs: &[(&str, usize)] = &[("MOCK_COM0", 0), ("MOCK_COM1", 1), ("MOCK_COM2", 2)];

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
                        if validate_adam(&frame) {
                            let fields = decode_adam(&frame);
                            let arc_fields = Arc::new(fields.clone());
                            shared_clone.adam_decoded.store(Arc::clone(&arc_fields));

                            let mut buf = [0u8; 256];
                            let n = frame.len().min(256);
                            buf[..n].copy_from_slice(&frame[..n]);
                            let sn = seq.fetch_add(1, Ordering::Relaxed);
                            shared_clone.adam_raw.update(sn, &buf);

                            let hex = crate::common::utils::format_hex_compact(&frame);
                            let event = Fj200cMainEvent::PortData {
                                connection_index: 1,
                                hex,
                                fields: Arc::new(ChannelData::Adam(fields)),
                            };
                            if let Ok(json) = crate::common::ws::serialize(&event) {
                                let _ = tx_clone.send(json);
                            }
                        }
                    }
                    2 => {
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
                                connection_index: 2,
                                hex,
                                fields: Arc::new(ChannelData::Dyno(fields)),
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

pub fn stop_mock_senders(stop: &AtomicBool) {
    stop.store(true, Ordering::Relaxed);
}
