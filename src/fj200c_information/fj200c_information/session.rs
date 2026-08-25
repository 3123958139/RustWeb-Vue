//! # 连接会话线程
//!
//! 每通道（连接）一个 IO 会话线程：读取数据 → 提取帧 → 解码 →
//! 推送事件 + 记录 CSV。从 fj200c_information.informatization 的 backend/session.rs 移植。
//!
//! ## 数据流
//!
//! ```text
//! control.recv() → extractor.feed() → 校验通过 → decode() → 28 字段
//!      │                                    │
//!      ├─ payload 事件（原始 hex，200ms 节流）├─ frame 事件（hex + 类型 + 字段）
//!      │                                    └─ table_data 事件（16 个 SharedData 字段）
//!      └─ SharedData 解码（frame[5..54]）  + CSV 写入（SYSJSK 开始 / SYSJMK 结束）
//! ```
//!
//! ## 实现要点
//!
//! - **解码结果传递**：解码器闭包（在 extractor 内部被调用）写入
//!   `Some(frame)`，主循环 `try_lock` 取走，避免阻塞
//! - **事件节流**：`payload` 与 `table_data` 事件每 200ms 最多发一次，
//!   避免高频数据导致前端 UI 卡顿
//! - **CSV 状态机**：`SYSJSK`（首块）创建 CSV → `SYSJZJK`（中间块）写行 →
//!   `SYSJMK`（末块）刷新关闭，与硬件试验数据下载协议对应
//! - **SharedData 16 字段解码**：非试验数据帧（参数设置/读取、遥测等）解码
//!   frame[5..54] 为产品名称、编号、累计时间等标识字段，供 table_data 推送

use crate::common::config::Config;
use crate::common::frame_extractor::FrameExtractor;
use crate::common::utils;
use crate::fj200c_information::csv_sink::CsvSink;
use crate::fj200c_information::decode::{
    frame_validator, make_decoder, ExtractedFrame, FrameType, CSV_HEADERS, FRAME_LEN, HEADER,
};
use crate::fj200c_information::frame_bundle::FrameBundle;
use crate::fj200c_information::mock::STOP_SIGNAL;
use crate::fj200c_information::state::SharedData;
use crate::fj200c_information::{Fj200cInformationEvent, IoControl, TableRow};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{info, trace, warn};

/// 全局帧数据复合存储（惰性初始化）
///
/// 所有连接会话线程共享此实例，更新最新帧数据供 WebSocket 快照使用。
pub static FRAME_BUNDLE: OnceLock<FrameBundle> = OnceLock::new();

/// 命令通道发送端（`service::send_command` 通过此通道发送命令到各会话线程）
pub static COMMAND_TX: OnceLock<Mutex<Option<mpsc::Sender<Vec<u8>>>>> = OnceLock::new();
/// 命令通道接收端（各会话线程从此接收待发送的命令）
pub static COMMAND_RX: OnceLock<Mutex<mpsc::Receiver<Vec<u8>>>> = OnceLock::new();

/// 初始化命令通道（幂等操作，多次调用安全）
///
/// 创建 `mpsc` 通道用于 `service::send_command` 向各会话线程发送命令。
pub fn init_command_channel() {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _ = COMMAND_TX.set(Mutex::new(Some(tx)));
    let _ = COMMAND_RX.set(Mutex::new(rx));
}

/// 表格数据推送节流间隔（200ms）
const TABLE_EMIT_INTERVAL: Duration = Duration::from_millis(200);
/// 原始数据推送节流间隔（200ms）
const PAYLOAD_EMIT_INTERVAL: Duration = Duration::from_millis(200);
/// 帧事件推送节流间隔（200ms，115200 波特率下每帧全量推送会打满广播通道）
const FRAME_EMIT_INTERVAL: Duration = Duration::from_millis(200);
/// 串口接收超时（毫秒）
const RECV_TIMEOUT_MS: u64 = 200;

/// 获取全局帧数据复合存储实例
pub fn frame_bundle() -> &'static FrameBundle {
    FRAME_BUNDLE.get_or_init(FrameBundle::new)
}

/// 运行单个连接的 IO 会话（阻塞，直至收到停止信号或 IO 错误）
///
/// 这是发动机监控模块的核心数据处理循环：
/// 1. 从 IO 设备接收原始字节
/// 2. 通过帧提取器定位完整帧
/// 3. 解码帧数据为 28 个工程字段
/// 4. 通过广播通道推送事件到 WebSocket
/// 5. 试验数据帧写入 CSV 文件
///
/// 参数：
/// - `connection_index`：连接索引（0~7），用于日志和事件标识
/// - `control`：IO 设备抽象（串口或模拟器）
/// - `tx`：广播通道发送端，推送事件到 WebSocket 客户端
/// - `cfg`：配置文件引用，读取 CSV 等设置
pub fn run_one_connection(
    connection_index: usize,
    control: Arc<dyn IoControl>,
    tx: broadcast::Sender<crate::common::ws::EventPayload>,
    cfg: &Config,
) {
    info!("会话线程启动: 连接 {} 开始读取数据", connection_index);

    // 从配置文件读取 CSV 相关设置
    let csv_enabled = cfg
        .get_or("CSV", "Enabled", "true")
        .eq_ignore_ascii_case("true");
    let csv_dir = cfg.get_or("CSV", "Dir", "csv");

    // 设置串口接收超时
    if let Err(e) = control.set_timeout(RECV_TIMEOUT_MS) {
        warn!("连接 {} 设置超时失败: {}", connection_index, e);
    }

    // 创建帧提取器（注入校验和解码闭包）
    let result: Arc<Mutex<Option<ExtractedFrame>>> = Arc::new(Mutex::new(None));
    let decoder = make_decoder(Arc::clone(&result));
    let mut extractor = FrameExtractor::new(
        HEADER.to_vec(),
        FRAME_LEN,
        Box::new(frame_validator),
        Box::new(decoder),
    );

    let start_time = Arc::new(Instant::now());
    let mut last_table_emit = Instant::now() - TABLE_EMIT_INTERVAL;
    let mut last_payload_emit = Instant::now() - PAYLOAD_EMIT_INTERVAL;
    let mut last_frame_emit = Instant::now() - FRAME_EMIT_INTERVAL;
    // CSV 磁盘 IO 移入独立写入线程（采集线程只做非阻塞 send，见 csv_sink.rs）
    let mut csv_sink: Option<CsvSink> =
        csv_enabled.then(|| CsvSink::start(csv_dir.clone(), connection_index));

    let mut recv_buf = [0u8; 512];

    loop {
        // 检查全局停止信号
        if STOP_SIGNAL.load(Ordering::Relaxed) {
            break;
        }

        // 处理待发送的命令（来自前端 send_command）
        if let Some(cmd_rx) = COMMAND_RX.get() {
            if let Ok(cmd) = cmd_rx.lock().unwrap_or_else(|e| e.into_inner()).try_recv() {
                match control.send(&cmd) {
                    Ok(_) => info!(
                        "连接 {} 发送命令: {}",
                        connection_index,
                        utils::format_hex(&cmd)
                    ),
                    Err(e) => warn!("连接 {} 发送命令失败: {}", connection_index, e),
                }
            }
        }

        match control.recv(&mut recv_buf) {
            Ok(n) if n > 0 => {
                let chunk = &recv_buf[..n];
                info!(
                    "连接 {} 收到 {} 字节: {}",
                    connection_index,
                    n,
                    utils::format_hex(chunk)
                );

                // payload 事件：原始数据（200ms 节流），预序列化后广播（只序列化一次）
                if last_payload_emit.elapsed() >= PAYLOAD_EMIT_INTERVAL {
                    let event = Fj200cInformationEvent::Payload {
                        connection_index,
                        hex: utils::format_hex(chunk),
                    };
                    if let Ok(json) = crate::common::ws::serialize(&event) {
                        let _ = tx.send(json);
                    }
                    last_payload_emit = Instant::now();
                }

                // 将原始数据送入帧提取器
                extractor.feed(chunk);

                // 非阻塞尝试取走解码结果
                if let Ok(mut guard) = result.try_lock() {
                    if let Some(extracted) = guard.take() {
                        handle_frame(
                            connection_index,
                            extracted,
                            &tx,
                            cfg,
                            &csv_sink,
                            &start_time,
                            &mut last_table_emit,
                            &mut last_frame_emit,
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                if STOP_SIGNAL.load(Ordering::Relaxed) {
                    break;
                }
                // 串口读取错误：Timeout 类错误视为正常轮询间隔，继续
                let err_str = e.to_string();
                if err_str.contains("timeout") || err_str.contains("timed out") {
                    continue;
                }
                warn!("连接 {} 读取失败: {}", connection_index, err_str);
                break;
            }
        }
    }

    // 会话结束：刷新尾帧并等待写入线程退出（保证 CSV 数据完整落盘）
    if let Some(sink) = csv_sink.take() {
        sink.shutdown();
    }
    info!("会话线程退出: 连接 {}", connection_index);
}

/// 处理单帧数据：解码、推送事件、记录 CSV
///
/// 根据帧类型执行不同的处理逻辑：
/// - `SYSJSK`（首块）：创建 CSV 文件
/// - `SYSJZJK`（中间块）：解码并写入 CSV
/// - `SYSJMK`（末块）：刷新并关闭 CSV
/// - 其他帧类型：解码 SharedData 标识字段
#[allow(clippy::too_many_arguments)]
fn handle_frame(
    connection_index: usize,
    extracted: ExtractedFrame,
    tx: &broadcast::Sender<crate::common::ws::EventPayload>,
    cfg: &Config,
    csv_sink: &Option<CsvSink>,
    start_time: &Arc<Instant>,
    last_table_emit: &mut Instant,
    last_frame_emit: &mut Instant,
) {
    let frame_type = match &extracted.frame_type {
        FrameType::CSSZZL => "参数设置",
        FrameType::CSDQZL => "参数读取",
        FrameType::SYSJXZZL => "试验数据下载",
        FrameType::SYSJSK => "试验数据首块",
        FrameType::SYSJZJK => "试验数据中间块",
        FrameType::SYSJMK => "试验数据末块",
        FrameType::JBCSQCZL => "基本参数清除",
        FrameType::SYSJQCZL => "试验数据清除",
        FrameType::NULL => "未知",
    };

    // 解码 28 个工程字段（只解码一次，CSV 写入与事件推送共用，避免重复解码）
    let fields = crate::fj200c_information::decode::decode(start_time.clone(), &extracted.data);

    // 试验数据下载状态机：首块创建 CSV，末块结束本次记录；
    // 其余帧类型解码 16 个 SharedData 标识字段（与源 session.rs 一致）
    match extracted.frame_type {
        FrameType::SYSJSK => {
            if let Some(sink) = csv_sink {
                info!("连接 {}: 试验数据首块，开始 CSV 记录", connection_index);
                let filename = format!(
                    "fj200c_information_{}.csv",
                    chrono::Local::now().format("%Y%m%d_%H%M%S")
                );
                sink.begin(
                    filename,
                    CSV_HEADERS.iter().map(|s| s.to_string()).collect(),
                );
            }
        }
        FrameType::SYSJZJK => {
            if let Some(sink) = csv_sink {
                sink.write_row(fields.clone());
            }
        }
        FrameType::SYSJMK => {
            if let Some(sink) = csv_sink {
                info!("连接 {}: 试验数据末块，结束 CSV 记录", connection_index);
                sink.end();
            }
        }
        _ => {
            decode_shared_data(&extracted.data);
        }
    }

    // 更新共享帧数据
    if !fields.is_empty() {
        frame_bundle().update(fields.clone(), &extracted.data);
    }

    let hex = utils::format_hex(&extracted.data);

    // frame 事件：供可视化/日志面板使用（200ms 节流，与 Payload/TableData 一致）
    if last_frame_emit.elapsed() >= FRAME_EMIT_INTERVAL {
        let event = Fj200cInformationEvent::Frame {
            connection_index,
            hex,
            frame_type: frame_type.to_string(),
            fields,
        };
        if let Ok(json) = crate::common::ws::serialize(&event) {
            let _ = tx.send(json);
        }
        *last_frame_emit = Instant::now();
    }

    // table_data 事件：16 个 SharedData 标识字段，200ms 节流
    if last_table_emit.elapsed() >= TABLE_EMIT_INTERVAL {
        let rows = shared_data_rows();
        let event = Fj200cInformationEvent::TableData {
            connection_index,
            rows,
        };
        trace!("{:?}", event);
        if let Ok(json) = crate::common::ws::serialize(&event) {
            let _ = tx.send(json);
        }
        *last_table_emit = Instant::now();
    }

    let _ = cfg; // 保留引用（后续可能按帧类型读取命令配置）
}

/// 从帧数据解码 16 个 SharedData 标识字段
///
/// `buf[i] = frame[4 + i]`，数据区从 frame[4] 开始。
/// 包括产品名称、编号、累计时间、指纹码等标识信息。
/// 解码结果写入全局 `SharedData` 单例，供 HTTP handler 读取推送。
fn decode_shared_data(frame: &[u8]) {
    if frame.len() < 49 {
        return;
    }
    let shared = SharedData::global();
    let mut ascii: [u8; 8] = [0; 8];

    ascii.copy_from_slice(&frame[5..13]);
    *shared
        .field_product_name
        .write()
        .unwrap_or_else(|e| e.into_inner()) =
        utils::little_endian_bytes_to_ascii(&ascii).unwrap_or_else(|_| "Err".into());

    ascii.copy_from_slice(&frame[13..21]);
    *shared
        .field_engine_product_code
        .write()
        .unwrap_or_else(|e| e.into_inner()) =
        utils::little_endian_bytes_to_ascii(&ascii).unwrap_or_else(|_| "Err".into());

    *shared
        .field_engine_factory_number
        .write()
        .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[21..25]);
    *shared
        .field_engine_test_date
        .write()
        .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[25..29]);

    ascii.copy_from_slice(&frame[29..37]);
    *shared
        .field_controller_product_code
        .write()
        .unwrap_or_else(|e| e.into_inner()) =
        utils::little_endian_bytes_to_ascii(&ascii).unwrap_or_else(|_| "Err".into());

    *shared
        .field_controller_number
        .write()
        .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[37..41]);
    *shared
        .field_gas_generator_number
        .write()
        .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[41..45]);
    // *shared
    //     .field_controller_power_on_seconds
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[42..44]);
    // *shared
    //     .field_controller_power_on_hours
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[44..48]);
    // *shared
    //     .field_engine_work_seconds
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[48..50]);
    // *shared
    //     .field_engine_work_hours
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[50..54]);
    //
    // *shared
    //     .field_engine_start_count
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) =
    //     format!("{}", frame[54] as u16 + frame[54] as u16 * 256u16);

    *shared
        .field_engine_software_fingerprint
        .write()
        .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[46..50]);
    // *shared
    //     .field_bootloader_fingerprint
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) = utils::format_hex(&frame[50..54]);

    // *shared
    //     .field_software_upgrade_count
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) =
    //     format!("{}", frame[64] as u16 + frame[65] as u16 * 256u16);
    //
    // *shared
    //     .field_power_on_count
    //     .write()
    //     .unwrap_or_else(|e| e.into_inner()) =
    //     format!("{}", frame[65] as u16 + frame[66] as u16 * 256u16);
}

/// 汇总 16 个 SharedData 字段为表格行（按字段名排序）
///
/// 使用 `push_row!` 宏简化重复的 RwLock 读取和 TableRow 构造。
/// 返回的 `Vec<TableRow>` 按字段名升序排列，与源 `emit_table_data_delta` 一致。
fn shared_data_rows() -> Vec<TableRow> {
    let shared = SharedData::global();
    let mut rows = Vec::new();

    /// 辅助宏：从 RwLock 字段读取值并构建 TableRow
    macro_rules! push_row {
        ($label:expr, $field:expr) => {
            if let Ok(val) = $field.read() {
                rows.push(TableRow {
                    field: $label.into(),
                    value: val.clone(),
                });
            }
        };
    }

    push_row!("产品名称", shared.field_product_name);
    push_row!("发动机产品代号", shared.field_engine_product_code);
    push_row!("发动机出厂编号", shared.field_engine_factory_number);
    push_row!("发动机检验试车日期", shared.field_engine_test_date);
    push_row!("电控器产品代号", shared.field_controller_product_code);
    push_row!("电控器编号", shared.field_controller_number);
    push_row!("燃气发生器编号", shared.field_gas_generator_number);
    // push_row!(
    //     "电控器加电累计时间（秒）",
    //     shared.field_controller_power_on_seconds
    // );
    // push_row!(
    //     "电控器加点累计时间（时）",
    //     shared.field_controller_power_on_hours
    // );
    // push_row!("发动机工作累计时间（秒）", shared.field_engine_work_seconds);
    // push_row!("发动机工作累计时间（时）", shared.field_engine_work_hours);
    // push_row!("发动机累计起动工作次数", shared.field_engine_start_count);
    push_row!(
        "发动机控制软件指纹码",
        shared.field_engine_software_fingerprint
    );
    // push_row!("bootloader指纹码", shared.field_bootloader_fingerprint);
    // push_row!("软件升级累计次数", shared.field_software_upgrade_count);
    // push_row!("通电工作累计次数", shared.field_power_on_count);

    rows.sort_by(|a, b| a.field.cmp(&b.field));
    rows
}
