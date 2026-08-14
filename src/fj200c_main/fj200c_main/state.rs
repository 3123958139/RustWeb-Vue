//! 全局运行时状态（fj200c_main 模块）
//!
//! 集中管理测控服务的运行期共享状态：
//! - 服务/录制/模拟/主题的运行标志（原子类型，跨线程可见）
//! - 共享端口数据（`SharedPortData`，ArcSwap 无锁读）
//! - CSV 录制写入器与 64 列表头字典
//! - ECU 指令发送线程的停止信号与最近发送数据
//!
//! 设计要点：标志位全部用原子类型（`AtomicBool`/`AtomicU8`），
//! 避免在采集高频线程中加锁；停止信号用 `Arc<AtomicBool>` 共享，
//! 由 `stop_service` 置位后线程自然退出。
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::common::csv_writer::CsvWriter;
use crate::fj200c_main::com::SharedPortData;

/// 配置文件路径（相对运行目录）
pub const CONFIG_PATH: &str = "config-fj200c_main.ini";

/// CSV 录制状态（0=停止，1=录制中）
pub static CSV_RECORDING: AtomicU8 = AtomicU8::new(0);
/// CSV 录制写入器（录制线程持有）
pub static CSV_WRITER: Mutex<Option<CsvWriter>> = Mutex::new(None);
/// 仪表盘主题（0=暗色，1=亮色；默认暗色 1）
pub static THEME_IS_DARK: AtomicU8 = AtomicU8::new(1);
/// 模拟运行标志（无需硬件时用模拟数据源）
pub static SIMULATION_MODE: AtomicBool = AtomicBool::new(false);
/// 模拟数据源发送线程的停止信号
pub static MOCK_SENDERS_STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
/// ECU 指令发送计数（帧序号，随发送递增，范围 0-255）
pub static ECU_SEND_COUNTER: AtomicU8 = AtomicU8::new(0);
/// 服务运行标志（start_service/stop_service 置位）
pub static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// ECU 最近发送的指令帧（十六进制字符串，供前端回显）
pub static ECU_SEND_DATA: OnceLock<ArcSwap<String>> = OnceLock::new();

/// 获取 ECU 最近发送数据（惰性初始化，默认空帧 `EB9010...`）
pub fn ecu_send_data() -> &'static ArcSwap<String> {
    ECU_SEND_DATA.get_or_init(|| {
        ArcSwap::new(Arc::new(String::from(
            "EB901000000000000000000000000000",
        )))
    })
}

/// 共享端口数据（五路串口最新帧的解码结果）
pub static SHARED_PORT_DATA: OnceLock<Arc<SharedPortData>> = OnceLock::new();

/// 获取共享端口数据（首次调用时惰性创建，保证未启动服务时模拟运行也能正常工作）
pub fn shared_port_data() -> Option<&'static Arc<SharedPortData>> {
    Some(SHARED_PORT_DATA.get_or_init(crate::fj200c_main::com::create_shared_port_data))
}

/// 已打开的五路串口句柄集合（服务运行时持有）
pub static ALL_COM_PORTS: Mutex<Option<crate::fj200c_main::com::AllComPorts>> = Mutex::new(None);
/// 帧处理线程的停止信号
pub static PROCESSING_STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// CSV 64 列表头字典：维护 键（英文字段名）→ （中文列名, 当前值） 的映射，
/// 同时按固定顺序缓存键/名数组，保证每行 CSV 列顺序稳定
pub struct CsvHeaderDict {
    keys: Vec<&'static str>,
    names: Vec<&'static str>,
    map: HashMap<&'static str, (&'static str, &'static str)>,
}

impl CsvHeaderDict {
    /// 从 `types::all_csv_entries()` 获取全部 (键, 中文列名) 条目
    fn entries() -> Vec<(&'static str, &'static str)> {
        crate::fj200c_main::types::all_csv_entries()
    }

    /// 构建字典：初始值为 0，按 entries 顺序固定键序
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

    /// 列数
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// 全部键（固定顺序）
    pub fn keys(&self) -> &[&'static str] {
        &self.keys
    }

    /// 全部中文列名（固定顺序）
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    /// 按键查中文列名
    pub fn get_name(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|&(name, _)| name)
    }

    /// 按键查当前值
    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|&(_, v)| v)
    }

    /// 写入某列当前值（值必须为 `&'static str`，通常来自解码后格式化的字符串）
    pub fn set_value(&mut self, key: &str, value: &'static str) {
        if let Some(entry) = self.map.get_mut(key) {
            entry.1 = value;
        }
    }
}

/// CSV 表头字典全局单例
static CSV_HEADER_DICT: OnceLock<RwLock<CsvHeaderDict>> = OnceLock::new();

/// 获取 CSV 表头字典（读写锁，录制线程写入、HTTP 处理器读取）
pub fn csv_header_dict() -> &'static RwLock<CsvHeaderDict> {
    CSV_HEADER_DICT.get_or_init(|| RwLock::new(CsvHeaderDict::new()))
}
