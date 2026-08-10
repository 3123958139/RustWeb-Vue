//! # CSV 文件写入器
//!
//! 将接收到的帧数据写入 CSV 文件，支持批量刷新和自动 Flush。
//! 从 dch crate（fj200c_information.informatization）移植。
//!
//! ## 关键语法
//!
//! - **`struct CsvWriter { inner: Mutex<CsvWriterInner>, ... }`**：
//!   内部可变性模式。对外提供 `&self` 方法（不可变引用），
//!   内部用 `Mutex` 实现可变访问。
//! - **`impl Drop for CsvWriter`**：对象销毁时自动 flush 缓存数据，
//!   确保程序退出时不丢失最后几条记录。
//! - **批量刷新策略**：首次写入立即 flush，后续每 500ms 批量刷新一次，
//!   平衡写入性能和数据安全性。

use std::fs::{create_dir_all, File, OpenOptions};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{error, trace};

/// 批量刷新间隔（毫秒）
const FLUSH_INTERVAL_MS: u64 = 500;

/// CSV 写入器内部状态（由 `Mutex` 保护）
struct CsvWriterInner {
    /// 底层 CSV 写入器
    writer: csv::Writer<File>,
    /// 待写入的数据行缓冲区
    buffer: Vec<Vec<String>>,
    /// 上次刷新时间戳
    last_flush: Instant,
    /// 是否已写入过数据（首次写入需立即 flush）
    has_written: bool,
}

impl CsvWriterInner {
    /// 将缓冲区中的所有数据行写入 CSV 文件并刷新磁盘缓存
    fn flush_batch(&mut self) -> Result<(), String> {
        for row in self.buffer.drain(..) {
            self.writer
                .write_record(row)
                .map_err(|e| format!("写入 CSV 失败: {}", e))?;
        }
        self.writer
            .flush()
            .map_err(|e| format!("刷新 CSV 失败: {}", e))?;
        self.last_flush = Instant::now();
        self.has_written = true;
        Ok(())
    }
}

/// CSV 文件写入器（线程安全）
///
/// 封装 CSV 文件的创建、写入和刷新操作。
/// 使用 `Mutex` 保护内部状态，支持多线程安全调用 `write_row`。
pub struct CsvWriter {
    /// 内部状态（互斥锁保护）
    inner: Mutex<CsvWriterInner>,
    /// CSV 文件路径
    path: PathBuf,
}

impl CsvWriter {
    /// 创建新的 CSV 文件并写入表头
    ///
    /// - `dir`：CSV 文件存储目录（不存在时自动创建）
    /// - `filename`：文件名（如 `"fj200c_information_20240101_120000.csv"`）
    /// - `headers`：CSV 表头列名列表
    pub fn create(dir: &str, filename: &str, headers: Vec<String>) -> Result<Self, String> {
        let dir_path = PathBuf::from(dir);
        create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {}", e))?;

        let path = dir_path.join(filename);

        // 以写入模式打开文件（创建或覆盖）
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| format!("打开文件失败: {}", e))?;

        let mut writer = csv::Writer::from_writer(file);
        writer
            .write_record(&headers)
            .map_err(|e| format!("写入 CSV 表头失败: {}", e))?;

        let inner = CsvWriterInner {
            writer,
            buffer: Vec::with_capacity(64), // 预分配 64 行缓冲区
            last_flush: Instant::now(),
            has_written: false,
        };

        Ok(Self {
            inner: Mutex::new(inner),
            path,
        })
    }

    /// 写入一行数据到 CSV 文件
    ///
    /// 数据先缓存在内存中，达到刷新条件时批量写入磁盘：
    /// - 首次写入立即 flush（确保表头不丢失）
    /// - 后续每 500ms 批量刷新一次
    pub fn write_row(&self, fields: Vec<String>) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;

        // 首次写入时强制 flush，确保表头已写入磁盘
        let should_flush = !inner.has_written;
        inner.buffer.push(fields);

        let elapsed = inner.last_flush.elapsed().as_millis() as u64;
        if should_flush || elapsed >= FLUSH_INTERVAL_MS {
            inner.flush_batch()?;
        } else {
            trace!("CSV 数据已缓存，{} 条待写入", inner.buffer.len());
        }

        Ok(())
    }

    /// 立即刷新缓冲区中的所有数据到磁盘
    pub fn flush(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        if !inner.buffer.is_empty() {
            inner.flush_batch()?;
        }
        Ok(())
    }

    /// 获取 CSV 文件的完整路径
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// 析构时自动刷新缓冲区，确保程序退出时不丢失最后几条记录
///
/// `Drop::drop` 是 Rust 的 RAII 机制，对象离开作用域时自动调用。
impl Drop for CsvWriter {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.inner.lock() {
            if !inner.buffer.is_empty() {
                if let Err(e) = inner.flush_batch() {
                    error!("CSV 写入器 Drop 时刷新失败: {}", e);
                }
            }
            let _ = inner.writer.flush();
        }
    }
}
