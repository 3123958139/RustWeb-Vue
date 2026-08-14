//! 串口抽象层：统一五路串口的打开、发送与接收线程
//!
//! 五路设备（ECU/Adam4015/Adam4117/测功机/流量计）协议差异仅体现在
//! 帧头/帧长/校验上，故抽象为 `ComSpec`（协议描述）与 `AbstractCom`
//! （串口句柄 + 接收线程）：接收线程用 `common/frame_extractor.rs` 的
//! `FrameExtractor` 按帧规约拆帧，拆出的完整帧交给解码器（`decode.rs`）。
//!
//! `DualCom` 为串口句柄实现（基于 serial2），承担 `IoControl` trait 的
//! 并发读写语义（详见其文档）。
use crate::common::frame_extractor::FrameExtractor;
use crate::common::io::IoControl;
use crate::fj200c_main::config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::sync::broadcast;
use tracing::{debug, error};

/// 接收缓冲长度（帧长 256 的两倍，一次读循环可吞下多帧）
pub const FRAME_LEN: usize = 256;

/// 串口协议描述：决定 FrameExtractor 如何从字节流中找帧边界
pub struct ComSpec {
    /// 配置节名（如 `[Connection1]`），串口参数从该节读取
    pub section: String,
    /// 连接序号（1 起，日志与状态区分用）
    pub conn_idx: usize,
    /// 帧头字节
    pub frame_header: Vec<u8>,
    /// 帧数据区长度（帧头之后到校验前的字节数）
    pub frame_data_len: usize,
    /// 帧尾长度（校验/结束字节数）
    pub frame_tail_len: usize,
}

impl ComSpec {
    /// ECU 协议（头 0xEB 0x90 0x2A，数据 38 字节 + 1 校验）
    pub fn ecu_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xEB, 0x90, 0x2A],
            frame_data_len: 38,
            frame_tail_len: 1,
        }
    }

    /// Adam4015 协议（ASCII 帧，头 `>`，数据 57 字节，无帧尾）
    pub fn adam4015_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![b'>'],
            frame_data_len: 57,
            frame_tail_len: 0,
        }
    }

    /// Adam4117 协议（与 4015 同为 ASCII `>` 帧，数据 57 字节）
    pub fn adam4117_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![b'>'],
            frame_data_len: 57,
            frame_tail_len: 0,
        }
    }

    /// 测功机协议（头 0xFF 0xFF，数据 14 字节 + 2 校验）
    pub fn dyno_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xFF, 0xFF],
            frame_data_len: 14,
            frame_tail_len: 2,
        }
    }

    /// 燃油流量计协议（与测功机同为 0xFF 0xFF 头，数据 14 字节 + 2 校验）
    pub fn flux_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xFF, 0xFF],
            frame_data_len: 14,
            frame_tail_len: 2,
        }
    }

    /// 完整帧长度 = 帧头 + 数据区 + 帧尾
    pub fn frame_size(&self) -> usize {
        self.frame_header.len() + self.frame_data_len + self.frame_tail_len
    }
}

/// 串口控制器（serial2 实现，支持并发读写）
///
/// 串口库换用 serial2：其在 Windows 上用 overlapped IO（读/写请求各自
/// 独立挂起，互不串行化），`read()/write()` 均只取 `&self`，收发两个线程
/// 可无锁并发调用。serialport 4.x 是同步 IO，阻塞读会串行化写请求，
/// 导致 100ms 周期发送被读超时周期卡成 100~700ms 抖动（serialport-rs
/// issue #29/#124 的已知行为）。
///
/// 流控固定关闭（`FlowControl::None`），防止 XOFF/XON 挂起写入。
pub struct DualCom {
    port: serial2::SerialPort,
    section: String,
}

impl DualCom {
    /// 按配置节参数打开串口（超时 100ms），失败返回错误信息
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: u8,
        section: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let port = crate::common::serial::open_port(
            port_name,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            100,
            section,
        )?;
        Ok(Self {
            port,
            section: section.to_string(),
        })
    }
}

impl IoControl for DualCom {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let n = self.port.write(buf).map_err(|e| {
            let msg = format!("[{}] 串口发送失败: {}", self.section, e);
            error!("{}", msg);
            msg
        })?;
        Ok(n)
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let n = self.port.read(buf).map_err(|e| {
            debug!("[{}] 串口接收超时: {}", self.section, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;
        Ok(n)
    }
}

/// 串口通信抽象：协议描述 + 串口句柄 + 停止标志
///
/// 对外只暴露 `send`（发指令）与 `start_with`（起接收线程）；
/// 接收线程按 `ComSpec` 拆帧，完整帧交给 `validator`/`decoder` 回调。
pub struct AbstractCom {
    com: Arc<dyn IoControl>,
    com_spec: Arc<ComSpec>,
    stop: Arc<AtomicBool>,
}

impl Drop for AbstractCom {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl AbstractCom {
    /// 从配置节（`[ConnectionN]`）读取串口参数并打开，返回自引用包装
    pub fn new(
        com_spec: ComSpec,
        stop: Arc<AtomicBool>,
        _tx: broadcast::Sender<crate::common::ws::EventPayload>,
    ) -> Result<Arc<Self>, String> {
        let section = com_spec.section.clone();
        let conn_idx = com_spec.conn_idx;

        let com_spec = Arc::new(com_spec);

        // 从配置读取端口与串口参数（缺省值：COM1 / 115200 / 8N1）
        let cfg = config::global()
            .ok_or("配置未加载")?
            .as_ref()
            .ok_or("配置未加载")?
            .clone();
        let port_name = cfg.get_or(&section, "PORTNAME", "COM1");
        let baud_rate = cfg
            .get_or(&section, "BaudRate", "115200")
            .parse::<u32>()
            .unwrap_or(115200);
        let data_bits = cfg
            .get_or(&section, "DataBits", "8")
            .parse::<u8>()
            .unwrap_or(8);
        let stop_bits = cfg
            .get_or(&section, "StopBits", "1")
            .parse::<u8>()
            .unwrap_or(1);
        let parity = cfg
            .get_or(&section, "Parity", "0")
            .parse::<u8>()
            .unwrap_or(0);

        let com: Arc<dyn IoControl> = match DualCom::new(
            &port_name,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            &section,
        ) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                error!("[COM-{}] {} 创建失败: {}", conn_idx, section, e);
                return Err(format!("{:?}", e));
            }
        };

        Ok(Arc::new(AbstractCom {
            com,
            com_spec,
            stop,
        }))
    }

    /// 发送原始字节（指令下发，帧由调用方组好）
    pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        self.com.send(buf)
    }

    /// 连接序号（用于日志与事件区分设备）
    pub fn conn_idx(&self) -> usize {
        self.com_spec.conn_idx
    }

    /// 启动接收线程：循环读取 → `FrameExtractor` 拆帧 → 逐帧调用
    /// `validator`（校验和）与 `decoder`（解码入共享状态）；`stop` 置位即退出
    pub fn start_with(
        self: &Arc<Self>,
        validator: impl Fn(&[u8]) -> bool + Send + Sync + 'static,
        decoder: impl Fn(&[u8]) -> bool + Send + Sync + 'static,
    ) {
        let com_spec = Arc::clone(&self.com_spec);
        let com = Arc::clone(&self.com);
        let stop = Arc::clone(&self.stop);
        thread::spawn(move || {
            let frame_size = com_spec.frame_size();
            let mut frame_extractor = FrameExtractor::new(
                com_spec.frame_header.clone(),
                frame_size,
                Box::new(validator),
                Box::new(decoder),
            );
            while !stop.load(Ordering::Relaxed) {
                let mut buf: [u8; FRAME_LEN * 2] = [0; FRAME_LEN * 2];
                match com.recv(&mut buf) {
                    Ok(len) => {
                        frame_extractor.feed(&buf[0..len]);
                    }
                    Err(_e) => {}
                }
            }
        });
    }
}
