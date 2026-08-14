use crate::common::frame_extractor::FrameExtractor;
use crate::common::io::IoControl;
use crate::fj200c_main::config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: u8,
        section: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut port = serial2::SerialPort::open(port_name, |mut settings: serial2::Settings| {
            settings.set_raw();
            settings.set_baud_rate(baud_rate)?;
            settings.set_char_size(match data_bits {
                5 => serial2::CharSize::Bits5,
                6 => serial2::CharSize::Bits6,
                7 => serial2::CharSize::Bits7,
                _ => serial2::CharSize::Bits8,
            });
            settings.set_stop_bits(match stop_bits {
                2 => serial2::StopBits::Two,
                _ => serial2::StopBits::One,
            });
            settings.set_parity(match parity {
                1 => serial2::Parity::Odd,
                2 => serial2::Parity::Even,
                _ => serial2::Parity::None,
            });
            settings.set_flow_control(serial2::FlowControl::None);
            Ok(settings)
        })
        .map_err(|e| {
            let msg = format!("[{}] 打开串口 {} 失败: {}", section, port_name, e);
            error!("{}", msg);
            msg
        })?;
        port.set_read_timeout(Duration::from_millis(100)).map_err(|e| {
            let msg = format!("[{}] 设置读超时失败: {}", section, e);
            error!("{}", msg);
            msg
        })?;
        info!(
            "[{}] 串口 {} 已打开(overlapped 并发读写, 无流控): {} baud, {}{}{}",
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
