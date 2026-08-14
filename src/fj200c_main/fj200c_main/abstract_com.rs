use crate::common::frame_extractor::FrameExtractor;
use crate::common::io::IoControl;
use crate::fj200c_main::config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

pub const FRAME_LEN: usize = 256;

pub struct ComSpec {
    pub section: String,
    pub conn_idx: usize,
    pub frame_header: Vec<u8>,
    pub frame_data_len: usize,
    pub frame_tail_len: usize,
}

impl ComSpec {
    pub fn ecu_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xEB, 0x90, 0x2A],
            frame_data_len: 38,
            frame_tail_len: 1,
        }
    }

    pub fn adam4015_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![b'>'],
            frame_data_len: 57,
            frame_tail_len: 0,
        }
    }

    pub fn adam4117_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![b'>'],
            frame_data_len: 57,
            frame_tail_len: 0,
        }
    }

    pub fn dyno_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xFF, 0xFF],
            frame_data_len: 14,
            frame_tail_len: 2,
        }
    }

    pub fn flux_protocol(section: &str, conn_idx: usize) -> Self {
        ComSpec {
            section: section.to_string(),
            conn_idx,
            frame_header: vec![0xFF, 0xFF],
            frame_data_len: 14,
            frame_tail_len: 2,
        }
    }

    pub fn frame_size(&self) -> usize {
        self.frame_header.len() + self.frame_data_len + self.frame_tail_len
    }
}

/// 读写分离串口控制器
///
/// 打开串口后用 `try_clone()` 复制句柄（Windows 下为 `DuplicateHandle`，
/// 得到两个独立句柄），读/写各持一把互斥锁互不阻塞。避免原先
/// 读写共用一把 Mutex 时，读线程阻塞在 100ms 读超时期间，
/// 写线程（100ms 周期发送）排队等锁造成卡顿延时。
///
/// 流控固定关闭（`FlowControl::None`），防止 XOFF/XON 挂起写入。
pub struct DualCom {
    reader: Mutex<Box<dyn serialport::SerialPort>>,
    writer: Mutex<Box<dyn serialport::SerialPort>>,
    section: String,
}

impl DualCom {
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: u8,
        section: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let writer = serialport::new(port_name, baud_rate)
            .data_bits(match data_bits {
                5 => serialport::DataBits::Five,
                6 => serialport::DataBits::Six,
                7 => serialport::DataBits::Seven,
                _ => serialport::DataBits::Eight,
            })
            .parity(match parity {
                1 => serialport::Parity::Odd,
                2 => serialport::Parity::Even,
                _ => serialport::Parity::None,
            })
            .stop_bits(match stop_bits {
                2 => serialport::StopBits::Two,
                _ => serialport::StopBits::One,
            })
            .timeout(Duration::from_millis(100))
            .flow_control(serialport::FlowControl::None)
            .open()
            .map_err(|e| {
                let msg = format!("[{}] 打开串口 {} 失败: {}", section, port_name, e);
                error!("{}", msg);
                msg
            })?;
        let reader = writer.try_clone().map_err(|e| {
            let msg = format!("[{}] 复制串口句柄 {} 失败: {}", section, port_name, e);
            error!("{}", msg);
            msg
        })?;
        info!(
            "[{}] 串口 {} 已打开(读写分离, 无流控): {} baud, {}{}{}",
            section,
            port_name,
            baud_rate,
            data_bits,
            match parity {
                1 => "O",
                2 => "E",
                _ => "N",
            },
            match stop_bits {
                2 => "2",
                _ => "1",
            },
        );
        Ok(Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            section: section.to_string(),
        })
    }
}

impl IoControl for DualCom {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        let n = writer.write(buf).map_err(|e| {
            let msg = format!("[{}] 串口发送失败: {}", self.section, e);
            error!("{}", msg);
            msg
        })?;
        Ok(n)
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let mut reader = self.reader.lock().unwrap_or_else(|e| e.into_inner());
        let n = reader.read(buf).map_err(|e| {
            debug!("[{}] 串口接收超时: {}", self.section, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;
        Ok(n)
    }

    fn set_timeout(&self, timeout_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = self.reader.lock().unwrap_or_else(|e| e.into_inner());
        reader
            .set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

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
    pub fn new(
        com_spec: ComSpec,
        stop: Arc<AtomicBool>,
        _tx: broadcast::Sender<crate::common::ws::EventPayload>,
    ) -> Result<Arc<Self>, String> {
        let section = com_spec.section.clone();
        let conn_idx = com_spec.conn_idx;

        let com_spec = Arc::new(com_spec);

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

    pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        self.com.send(buf)
    }

    pub fn conn_idx(&self) -> usize {
        self.com_spec.conn_idx
    }

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
