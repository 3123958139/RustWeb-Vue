use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::common::csv_writer::CsvWriter;
use crate::fj200c_main::com::SharedPortData;

pub const CONFIG_PATH: &str = "config-fj200c_main.ini";

pub static CSV_RECORDING: AtomicU8 = AtomicU8::new(0);
pub static CSV_WRITER: Mutex<Option<CsvWriter>> = Mutex::new(None);
pub static THEME_IS_DARK: AtomicU8 = AtomicU8::new(1);
pub static SIMULATION_MODE: AtomicBool = AtomicBool::new(false);
pub static MOCK_SENDERS_STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
pub static ECU_SEND_COUNTER: AtomicU8 = AtomicU8::new(0);
pub static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

pub static ECU_SEND_DATA: OnceLock<ArcSwap<String>> = OnceLock::new();

pub fn ecu_send_data() -> &'static ArcSwap<String> {
    ECU_SEND_DATA.get_or_init(|| {
        ArcSwap::new(Arc::new(String::from(
            "EB901000000000000000000000000000",
        )))
    })
}

pub static SHARED_PORT_DATA: OnceLock<Arc<SharedPortData>> = OnceLock::new();

/// 获取共享端口数据（首次调用时惰性创建，保证未启动服务时模拟运行也能正常工作）
pub fn shared_port_data() -> Option<&'static Arc<SharedPortData>> {
    Some(SHARED_PORT_DATA.get_or_init(crate::fj200c_main::com::create_shared_port_data))
}

pub static ALL_COM_PORTS: Mutex<Option<crate::fj200c_main::com::AllComPorts>> = Mutex::new(None);
pub static PROCESSING_STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

pub struct CsvHeaderDict {
    keys: Vec<&'static str>,
    names: Vec<&'static str>,
    map: HashMap<&'static str, (&'static str, &'static str)>,
}

impl CsvHeaderDict {
    fn entries() -> Vec<(&'static str, &'static str)> {
        crate::fj200c_main::types::all_csv_entries()
    }

    fn new() -> Self {
        let entries = Self::entries();
        let mut keys = Vec::with_capacity(entries.len());
        let mut names = Vec::with_capacity(entries.len());
        let mut map = HashMap::with_capacity(entries.len());
        for (k, n) in entries {
            keys.push(k);
            names.push(n);
            map.insert(k, (n, "0"));
        }
        Self { keys, names, map }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn keys(&self) -> &[&'static str] {
        &self.keys
    }

    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    pub fn get_name(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|&(name, _)| name)
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|&(_, v)| v)
    }

    pub fn set_value(&mut self, key: &str, value: &'static str) {
        if let Some(entry) = self.map.get_mut(key) {
            entry.1 = value;
        }
    }
}

static CSV_HEADER_DICT: OnceLock<RwLock<CsvHeaderDict>> = OnceLock::new();

pub fn csv_header_dict() -> &'static RwLock<CsvHeaderDict> {
    CSV_HEADER_DICT.get_or_init(|| RwLock::new(CsvHeaderDict::new()))
}
