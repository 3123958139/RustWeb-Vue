//! # 遥测 CSV 录制
//!
//! 其他 8 个角色模块均有 CSV 录制，qgc 补齐：接收线程每次广播遥测
//! （10Hz，`[Gcs] TelemetryHz` 配置）时写入一行，文件按天分割
//! （`qgc_telemetry_YYYYMMDD.csv`，`[CSV] Dir` 配置目录，缺省 `csv/qgc`）。
//!
//! 实现要点：
//!
//! - 复用公共 `CsvWriter`（内存缓冲 + 定时批量 flush），录制行不阻塞
//!   UDP 接收循环（每行仅内存追加，落盘由内部节流）
//! - 全局单例持有当前文件；日期变化时自动重建（跨天续录）
//! - `[CSV] Enabled = false` 关闭录制（接收线程不再调用本模块）

use crate::common::csv_writer::CsvWriter;
use crate::qgc::models::QgcTelemetry;
use std::sync::{Mutex, OnceLock};

/// 表头列：时间 / 模式 / 状态 / 位置 / 姿态 / 速度 / 电量 / 油门 / GPS / 时长
const HEADERS: &[&str] = &[
    "timestamp", "mode", "armed", "lat", "lon", "alt_msl", "alt_rel", "heading",
    "roll", "pitch", "groundspeed", "airspeed", "climb", "voltage", "current",
    "battery_remaining", "battery_consumed_mah", "throttle", "gps_fix_type",
    "satellites_visible", "packet_rate", "flight_time_s",
];

/// 录制器状态（单例，接收线程独占访问）
struct Recorder {
    writer: Option<CsvWriter>,
    day: String,
}

static RECORDER: OnceLock<Mutex<Recorder>> = OnceLock::new();

fn recorder() -> &'static Mutex<Recorder> {
    RECORDER.get_or_init(|| Mutex::new(Recorder {
        writer: None,
        day: String::new(),
    }))
}

/// 当前日期（本地时区，YYYYMMDD，用于按天分割文件名）
fn today() -> String {
    chrono::Local::now().format("%Y%m%d").to_string()
}

/// 写入一行遥测（接收线程广播处调用）
///
/// `[CSV] Enabled = false` 时静默返回；文件打开失败仅记录日志，
/// 录制失败不影响遥测主流程。
pub fn record(t: &QgcTelemetry) {
    if !crate::qgc::config::csv_enabled() {
        return;
    }
    let mut rec = recorder().lock().unwrap_or_else(|e| e.into_inner());
    let day = today();
    // 按天分割：日期变化时重建文件（跨天续录）
    if rec.day != day {
        rec.day = day.clone();
        let headers = HEADERS.iter().map(|s| s.to_string()).collect();
        rec.writer = CsvWriter::create(&crate::qgc::config::csv_dir(), &format!("qgc_telemetry_{day}.csv"), headers).ok();
        if rec.writer.is_none() {
            tracing::warn!("[qgc] 遥测 CSV 文件创建失败（{} 目录不可写？）", crate::qgc::config::csv_dir());
        }
    }
    let Some(w) = &rec.writer else { return };
    let _ = w.write_row(vec![
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        t.mode.clone(),
        if t.armed { "1" } else { "0" }.to_string(),
        format!("{:.7}", t.lat),
        format!("{:.7}", t.lon),
        format!("{:.2}", t.altitude),
        format!("{:.2}", t.relative_alt),
        format!("{:.1}", t.heading),
        format!("{:.2}", t.roll),
        format!("{:.2}", t.pitch),
        format!("{:.2}", t.groundspeed),
        format!("{:.2}", t.airspeed),
        format!("{:.2}", t.climb),
        format!("{:.2}", t.voltage),
        format!("{:.2}", t.current),
        t.battery_remaining.to_string(),
        format!("{:.1}", t.battery_consumed_mah),
        format!("{:.0}", t.throttle),
        t.gps_fix_type.to_string(),
        t.satellites_visible.to_string(),
        format!("{:.1}", t.packet_rate),
        format!("{:.1}", t.flight_time_s),
    ]);
}