//! # CSV 文件写入器
//!
//! 将帧数据写入 CSV 文件，支持批量缓存（500ms）和自动 Flush（Drop）。

use std::fs::{create_dir_all, File, OpenOptions};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{error, trace};

const FLUSH_INTERVAL_MS: u64 = 500;

struct CsvWriterInner {
    writer: csv::Writer<File>,
    buffer: Vec<Vec<String>>,
    last_flush: Instant,
    has_written: bool,
}

impl CsvWriterInner {
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

pub struct CsvWriter {
    inner: Mutex<CsvWriterInner>,
    path: PathBuf,
}

impl CsvWriter {
    pub fn create(dir: &str, filename: &str, headers: Vec<String>) -> Result<Self, String> {
        let dir_path = PathBuf::from(dir);
        create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {}", e))?;

        let path = dir_path.join(filename);

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
            buffer: Vec::with_capacity(64),
            last_flush: Instant::now(),
            has_written: false,
        };

        Ok(Self {
            inner: Mutex::new(inner),
            path,
        })
    }

    pub fn write_row(&self, fields: Vec<String>) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;

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

    pub fn flush(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        if !inner.buffer.is_empty() {
            inner.flush_batch()?;
        }
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

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
