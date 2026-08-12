//! 二级子模块：协议生成器核心逻辑。
//!
//! 从 demo-protocol（Tauri 桌面应用）`src-tauri/src/protocol.rs` 迁移，
//! 适配 Web 架构：
//! - Excel 导出：不再落盘到任意路径，改为 `save_to_buffer` 返回二进制流
//!   （前端 `Blob` 下载），sheet 名称 / 列宽 / 格式与原实现保持一致
//! - 默认参数表：存服务器运行目录 `parameters.csv`（UTF-8 BOM + 种子内容），
//!   替代原 `app_data_dir` 方案
//! - CSV 解析 / 序列化：保留 Rust `csv` crate 实现，通过 HTTP 端点暴露，
//!   替代前端直接读写任意本地路径

use std::path::{Path, PathBuf};

use super::models::{ParameterDef, ProtocolField};

/// CSV 表头（写出的固定列顺序）
const CSV_HEADERS: [&str; 5] = ["参数名称", "别名", "单位", "数据类型", "备注"];

/// 默认参数表种子内容（首次运行时写入，UTF-8 BOM）
const DEFAULT_CSV_CONTENT: &str = "\u{FEFF}参数名称,别名,单位,数据类型,备注
系统电压,SysVoltage,V,ushort,系统供电电压
系统电流,SysCurrent,A,float,系统工作电流
运行状态,RunStatus,,byte,0停止 1运行 2故障
设备温度,DeviceTemp,℃,short,传感器采集温度
累计运行时间,RunTime,h,uint,累计运行小时数
固件版本,FwVersion,,string,主控固件版本号
错误代码,ErrCode,,ushort,最近一次故障代码
";

/// 生成 Markdown 协议表文本
pub fn export_markdown(title: &str, data: &[ProtocolField]) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str("| 序号 | 字节范围 | 参数名称 | 单位 | 数据类型 | 备注 |\n");
    md.push_str("|------|----------|----------|------|----------|------|\n");
    for field in data {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            field.index, field.byte_range, field.name, field.unit, field.data_type, field.remark
        ));
    }
    md
}

/// 生成 Excel 协议表二进制（xlsx，内存缓冲，前端 Blob 下载）
pub fn export_excel(title: &str, data: &[ProtocolField]) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};

    let mut workbook = Workbook::new();

    let mut sheet_name: String = title
        .chars()
        .filter(|c| !matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
        .collect();
    if sheet_name.trim().is_empty() {
        sheet_name = "通信协议表".to_string();
    }
    if sheet_name.chars().count() > 31 {
        sheet_name = sheet_name.chars().take(31).collect();
    }

    let worksheet = workbook
        .add_worksheet()
        .set_name(&sheet_name)
        .map_err(|e| e.to_string())?;

    let title_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_align(FormatAlign::Center);
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE8E8E8))
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);
    let cell_format = Format::new()
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    worksheet
        .merge_range(0, 0, 0, 5, &sheet_name, &title_format)
        .map_err(|e| e.to_string())?;

    let headers = ["序号", "字节范围", "参数名称", "单位", "数据类型", "备注"];
    for (col, h) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(1, col as u16, *h, &header_format)
            .map_err(|e| e.to_string())?;
    }

    for (i, field) in data.iter().enumerate() {
        let row = (i + 2) as u32;
        worksheet
            .write_number_with_format(row, 0, field.index, &cell_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 1, &field.byte_range, &cell_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 2, &field.name, &cell_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 3, &field.unit, &cell_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 4, &field.data_type, &cell_format)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 5, &field.remark, &cell_format)
            .map_err(|e| e.to_string())?;
    }

    worksheet.set_row_height(0, 24.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(0, 6.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(1, 12.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(2, 22.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(3, 8.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(4, 14.0).map_err(|e| e.to_string())?;
    worksheet.set_column_width(5, 24.0).map_err(|e| e.to_string())?;

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

/// 默认参数表路径（服务器运行目录 `parameters.csv`）
pub fn default_csv_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("parameters.csv")
}

/// 解析 CSV 文本（按中文表头定位列，兼容 UTF-8 BOM）为参数表
pub fn parse_csv_content(content: &str) -> Result<Vec<ParameterDef>, String> {
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let mut rdr = csv::ReaderBuilder::new().from_reader(content.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let name_idx = col("参数名称");
    let alias_idx = col("别名");
    let unit_idx = col("单位");
    let type_idx = col("数据类型");
    let remark_idx = col("备注");
    let cell = |rec: &csv::StringRecord, idx: Option<usize>| {
        idx.and_then(|i| rec.get(i)).unwrap_or("").trim().to_string()
    };
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| e.to_string())?;
        out.push(ParameterDef {
            name: cell(&rec, name_idx),
            alias: cell(&rec, alias_idx),
            unit: cell(&rec, unit_idx),
            data_type: cell(&rec, type_idx),
            remark: cell(&rec, remark_idx),
        });
    }
    Ok(out)
}

/// 将参数表序列化为 CSV 文本（UTF-8 BOM，供前端下载）
pub fn serialize_csv(data: &[ParameterDef]) -> Result<String, String> {
    let mut wtr = csv::WriterBuilder::new().from_writer(Vec::new());
    wtr.write_record(CSV_HEADERS).map_err(|e| e.to_string())?;
    for d in data {
        wtr.write_record([
            d.name.as_str(),
            d.alias.as_str(),
            d.unit.as_str(),
            d.data_type.as_str(),
            d.remark.as_str(),
        ])
        .map_err(|e| e.to_string())?;
    }
    let mut content = wtr.into_inner().map_err(|e| e.to_string())?;
    let mut full = Vec::with_capacity(content.len() + 3);
    full.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    full.append(&mut content);
    String::from_utf8(full).map_err(|e| e.to_string())
}

/// 读取默认参数表（不存在时写入种子内容）
pub fn load_default_csv() -> Result<Vec<ParameterDef>, String> {
    let path = default_csv_path();
    if !path.exists() {
        std::fs::write(&path, DEFAULT_CSV_CONTENT).map_err(|e| e.to_string())?;
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    parse_csv_content(&text)
}

/// 保存默认参数表到服务器磁盘
pub fn save_default_csv(data: &[ParameterDef]) -> Result<(), String> {
    let content = serialize_csv(data)?;
    std::fs::write(default_csv_path(), content).map_err(|e| e.to_string())
}

/// 从磁盘文件读取 CSV（保留内部函数，供测试/扩展使用）
#[allow(dead_code)]
fn read_csv_file(path: &Path) -> Result<Vec<ParameterDef>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&bytes);
    let text = String::from_utf8_lossy(bytes);
    parse_csv_content(&text)
}