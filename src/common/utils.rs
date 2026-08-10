//! # 公共工具函数
//!
//! 各角色模块共用的字符串/字节处理工具（原分散于 fj200c_information/ftj1c 的 utils.rs）。

/// 将十六进制字符串解析为字节数组
///
/// 支持大写和小写，要求长度为偶数，自动去除首尾空白。
pub fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 || s.is_empty() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

/// 字节值 → 两位大写十六进制字符的静态查表（256 项 × 2 字符）
///
/// 热点路径（帧 hex 格式化）每帧调用数百次 `format!("{:02X}")`，
/// 查表比格式化快 5-10 倍。`const fn` 保证编译期生成，零运行时开销。
static HEX_UPPER: [u8; 512] = build_hex_table();

const fn build_hex_table() -> [u8; 512] {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut table = [0u8; 512];
    let mut i = 0usize;
    while i < 256 {
        table[i * 2] = DIGITS[i >> 4];
        table[i * 2 + 1] = DIGITS[i & 0x0F];
        i += 1;
    }
    table
}

/// 将字节数组格式化为大写十六进制字符串（空格分隔）
///
/// 预分配容量，查表转换（见 `HEX_UPPER`），避免逐字节 `format!`。
pub fn format_hex(data: &[u8]) -> String {
    let mut hex = String::with_capacity(data.len() * 3);
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            hex.push(' ');
        }
        let idx = (*b as usize) << 1;
        hex.push(HEX_UPPER[idx] as char);
        hex.push(HEX_UPPER[idx + 1] as char);
    }
    hex
}

/// 将字节数组格式化为连续大写十六进制字符串（无分隔符，查表实现）
pub fn format_hex_compact(data: &[u8]) -> String {
    let mut hex = String::with_capacity(data.len() * 2);
    for &b in data {
        let idx = (b as usize) << 1;
        hex.push(HEX_UPPER[idx] as char);
        hex.push(HEX_UPPER[idx + 1] as char);
    }
    hex
}

/// 从自定义 IP 标识解析端口名
///
/// 输入 `"IP8080"` → 输出 `Some("Port8080")`，用于从配置键名派生对应端口键名。
pub fn parse_ip_to_port(input: &str) -> Option<String> {
    if !input.starts_with("IP") {
        return None;
    }
    let mut chars = input.chars();
    chars.next(); // 跳过 'I'
    chars.next(); // 跳过 'P'

    let mut port_str = String::new();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            port_str.push(c);
        } else {
            break;
        }
    }

    if port_str.is_empty() {
        return None;
    }

    Some(format!("Port{}", port_str))
}

/// 将小端 8 字节数组转换为 ASCII 字符串（跳过 0 填充字节）
pub fn little_endian_bytes_to_ascii(bytes: &[u8; 8]) -> Result<String, String> {
    for &byte in bytes {
        if byte != 0 && byte > 127 {
            return Err(format!("非 ASCII 字符：0x{:02x}", byte));
        }
    }

    let mut chars = Vec::new();
    for &byte in bytes {
        if byte != 0 {
            chars.push(byte as char);
        }
    }

    Ok(chars.into_iter().collect())
}

/// 获取当前毫秒时间戳（自 Unix 纪元以来）
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 从字符串中提取第一个连续数字序列（如 `"COM10"` → `Some(10)`）
pub fn extract_number(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let start_idx = bytes.iter().position(|&b| b.is_ascii_digit())?;
    let end_idx = bytes[start_idx..]
        .iter()
        .position(|&b| !b.is_ascii_digit())
        .map_or(bytes.len(), |offset| start_idx + offset);
    let num_str = &s[start_idx..end_idx];
    num_str.parse::<usize>().ok()
}

/// 将文件名追加后缀并拼接 GlobalVar 中 PathCSV 指定的目录
pub fn rename_file_name(path: &str, new: &str) -> String {
    let gv = crate::common::global_var::GlobalVar::global().unwrap();
    let (name, ext) = path.split_once('.').unwrap_or((path, ""));
    format!(
        "{}/{}_{}.{}",
        gv.get("PathCSV").unwrap(),
        name,
        new,
        ext
    )
}

/// 读取无表头 CSV 为 HashMap（第一列为 key，第二列为 value）
pub fn read_csv_to_map(
    path: &str,
) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);

    let mut map = std::collections::HashMap::new();
    for result in reader.records() {
        let record = result?;
        if record.get(0).map_or(false, |k| k.is_empty()) {
            continue;
        }
        let key = record.get(0).unwrap_or("").to_string();
        let value = record.get(1).unwrap_or("").to_string();
        map.insert(key, value);
    }

    Ok(map)
}

/// 将 ASCII 字符串转为小端 8 字节数组（高地址存首字符，低地址填 0）
pub fn ascii_to_little_endian_bytes(input: &str) -> Result<[u8; 8], String> {
    if input.is_empty() {
        return Ok([0u8; 8]);
    }

    let bytes = input.as_bytes();
    if bytes.len() > 8 {
        return Err(format!(
            "输入字符串长度超过 8 个字符，当前长度：{}",
            bytes.len()
        ));
    }

    let mut result = [0u8; 8];
    for (i, &byte) in bytes.iter().enumerate() {
        if byte > 127 {
            return Err(format!("非 ASCII 字符：0x{:02x}", byte));
        }
        result[result.len() - 1 - i] = byte;
    }

    Ok(result)
}
