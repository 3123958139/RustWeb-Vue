//! # 发动机模块全局状态
//!
//! 存放服务运行标志、配置文件路径和 16 个解码字段的共享数据。
//!
//! ## 关键语法
//!
//! - **`OnceLock<SharedData>`**：全局单例。会话线程通过 `SharedData::global()`
//!   获取共享引用并写入解码字段，HTTP handler 读取后推送表格事件。
//! - **`AtomicBool`**：原子布尔标志，多线程无锁读写服务运行状态。
//!   `Ordering::Relaxed` 表示最宽松的内存序，适合仅需最终一致性的场景。
//! - **`RwLock<T>`**：读写锁，多个线程可同时读（`.read()`），
//!   但写入需要独占（`.write()`），适合读多写少的场景。

use std::sync::atomic::AtomicBool;
use std::sync::{OnceLock, RwLock};

/// 服务运行状态标志（全局原子布尔）
///
/// `true` 表示发动机监控服务正在运行，`false` 表示已停止。
/// HTTP handler 通过此标志快速判断是否允许发送命令等操作。
pub static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// 配置文件路径（相对于工作目录）
pub const CONFIG_PATH: &str = "config-fj200c_information.ini";

/// 发动机参数的 16 个标识字段共享数据
///
/// 每个字段用 `RwLock<String>` 包装，支持多线程并发读写：
/// - 会话线程解码帧数据后写入对应字段
/// - HTTP handler 读取字段值构建表格事件推送给前端
///
/// 字段默认值为中文描述文本，解码后替换为实际值。
pub struct SharedData {
    pub field_product_name: RwLock<String>,
    pub field_engine_product_code: RwLock<String>,
    pub field_engine_factory_number: RwLock<String>,
    pub field_engine_test_date: RwLock<String>,
    pub field_controller_product_code: RwLock<String>,
    pub field_controller_number: RwLock<String>,
    pub field_gas_generator_number: RwLock<String>,
    // pub field_controller_power_on_seconds: RwLock<String>,
    // pub field_controller_power_on_hours: RwLock<String>,
    // pub field_engine_work_seconds: RwLock<String>,
    // pub field_engine_work_hours: RwLock<String>,
    // pub field_engine_start_count: RwLock<String>,
    pub field_engine_software_fingerprint: RwLock<String>,
    pub field_bootloader_fingerprint: RwLock<String>,
    // pub field_software_upgrade_count: RwLock<String>,
    // pub field_power_on_count: RwLock<String>,
}

impl SharedData {
    /// 创建新的 SharedData 实例（各字段初始化为中文描述文本）
    fn new() -> Self {
        Self {
            field_product_name: RwLock::new(String::from("产品名称")),
            field_engine_product_code: RwLock::new(String::from("发动机产品代号")),
            field_engine_factory_number: RwLock::new(String::from("发动机出厂编号")),
            field_engine_test_date: RwLock::new(String::from("发动机检验试车日期")),
            field_controller_product_code: RwLock::new(String::from("电控器产品代号")),
            field_controller_number: RwLock::new(String::from("电控器编号")),
            field_gas_generator_number: RwLock::new(String::from("燃气发生器编号")),
            // field_controller_power_on_seconds: RwLock::new(String::from("电控器加电累计时间（秒）")),
            // field_controller_power_on_hours: RwLock::new(String::from("电控器加点累计时间（时）")),
            // field_engine_work_seconds: RwLock::new(String::from("发动机工作累计时间（秒）")),
            // field_engine_work_hours: RwLock::new(String::from("发动机工作累计时间（时）")),
            // field_engine_start_count: RwLock::new(String::from("发动机累计起动工作次数")),
            field_engine_software_fingerprint: RwLock::new(String::from("发动机控制软件指纹码")),
            field_bootloader_fingerprint: RwLock::new(String::from("bootloader指纹码")),
            // field_software_upgrade_count: RwLock::new(String::from("软件升级累计次数")),
            // field_power_on_count: RwLock::new(String::from("通电工作累计次数")),
        }
    }

    /// 获取全局 SharedData 单例引用（惰性初始化）
    ///
    /// 使用 `OnceLock` 保证线程安全的一次性初始化，
    /// 所有线程首次调用时创建实例，后续调用直接返回引用。
    pub fn global() -> &'static SharedData {
        static GLOBAL: OnceLock<SharedData> = OnceLock::new();
        GLOBAL.get_or_init(SharedData::new)
    }
}
