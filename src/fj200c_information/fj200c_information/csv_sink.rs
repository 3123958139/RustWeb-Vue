//! # CSV 异步写入器（磁盘 IO 移出采集线程）
//!
//! 原实现：CSV 文件的创建 / 写入 / flush 全部在串口采集线程内同步执行，
//! 磁盘 IO 可能阻塞帧处理（每帧最多一次 `write_row` + 底层 BufWriter flush）。
//!
//! 本模块用 `mpsc` 通道 + 独立 `std::thread` 解耦：
//!
//! ```text
//! 采集线程 ──(非阻塞 send)──▶ CsvSink ──▶ 写入线程（文件创建/写行/flush）
//! ```
//!
//! - 采集线程只做内存拷贝（发送 `Vec<String>`），不再触碰磁盘
//! - 写入线程串行处理所有命令，`Shutdown` 时 flush 尾帧并退出（线程 join）
//! - `CsvWriter` 内部为 `Mutex<CsvWriterInner>`，本就线程安全，可整体搬入写入线程

use crate::common::csv_writer::CsvWriter;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{error, info};

/// 写入线程指令
enum CsvCommand {
    /// 试验数据首块：创建 CSV 文件（带表头）
    Begin { filename: String, headers: Vec<String> },
    /// 试验数据中间块：写入一行
    Row(Vec<String>),
    /// 试验数据末块：刷新并关闭当前文件
    End,
    /// 会话结束：刷新并退出线程
    Shutdown,
}

/// CSV 异步写入器句柄（会话线程持有）
pub struct CsvSink {
    tx: Sender<CsvCommand>,
    handle: Option<JoinHandle<()>>,
}

impl CsvSink {
    /// 启动写入线程
    ///
    /// 线程创建失败时仅记录日志（罕见边缘场景），
    /// 后续 send 静默失败，采集线程绝不 panic/阻塞。
    pub fn start(dir: String, connection_index: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = match thread::Builder::new()
            .name(format!("csv-writer-{}", connection_index))
            .spawn(move || writer_loop(connection_index, &dir, rx))
        {
            Ok(handle) => Some(handle),
            Err(e) => {
                error!("连接 {} CSV 写入线程创建失败: {}", connection_index, e);
                None
            }
        };
        CsvSink { tx, handle }
    }

    /// 试验数据首块：请求创建 CSV 文件（非阻塞，磁盘 IO 在写入线程）
    pub fn begin(&self, filename: String, headers: Vec<String>) {
        let _ = self.tx.send(CsvCommand::Begin { filename, headers });
    }

    /// 写一行（非阻塞，磁盘 IO 在写入线程）
    pub fn write_row(&self, fields: Vec<String>) {
        let _ = self.tx.send(CsvCommand::Row(fields));
    }

    /// 试验数据末块：刷新并关闭当前文件
    pub fn end(&self) {
        let _ = self.tx.send(CsvCommand::End);
    }

    /// 会话结束：flush 尾帧并等待写入线程退出（保证 CSV 数据完整落盘）
    pub fn shutdown(mut self) {
        let _ = self.tx.send(CsvCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 写入线程主循环：串行处理文件创建 / 写行 / flush / 关闭
fn writer_loop(connection_index: usize, dir: &str, rx: Receiver<CsvCommand>) {
    let mut csv: Option<CsvWriter> = None;
    for cmd in rx {
        match cmd {
            CsvCommand::Begin { filename, headers } => {
                match CsvWriter::create(dir, &filename, headers) {
                    Ok(writer) => {
                        info!("CSV 文件已创建: {}", writer.path().display());
                        csv = Some(writer);
                    }
                    Err(e) => {
                        error!("连接 {} 创建 CSV 文件失败: {}", connection_index, e);
                        csv = None;
                    }
                }
            }
            CsvCommand::Row(fields) => {
                if let Some(writer) = &mut csv {
                    if let Err(e) = writer.write_row(fields) {
                        error!("连接 {} 写入 CSV 失败: {}", connection_index, e);
                    }
                }
            }
            CsvCommand::End => {
                if let Some(writer) = &mut csv {
                    let _ = writer.flush();
                }
                csv = None;
            }
            CsvCommand::Shutdown => {
                if let Some(writer) = &mut csv {
                    let _ = writer.flush();
                }
                break;
            }
        }
    }
}
