use crate::common::frame_extractor::FrameExtractor;
use crate::common::io::IoControl;
use crate::fj200c_information::com::ComControl;
use crate::fj200c_main::config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::sync::broadcast;
use tracing::error;

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

        let cfg = config::global().ok_or("配置未加载")?;
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
        let flow_control = cfg
            .get_or(&section, "FlowControl", "false")
            .eq_ignore_ascii_case("true");

        let com: Arc<dyn IoControl> = match ComControl::new(
            &port_name,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
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
