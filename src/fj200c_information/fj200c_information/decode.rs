//! # 帧解码模块
//!
//! 负责校验原始帧数据，并将其解码为可读的工程字段。
//! 从 fj200c_information.informatization 的 backend/decode.rs 移植（100 字节帧协议）。
//!
//! ## 帧格式
//!
//! 帧固定 100 字节，实际遥测数据在前 50 字节：
//! ```text
//! [0..3]    帧头 0xEB 0x90 0x64
//! [3]       帧类型字节（见下表）
//! [4..50]   遥测数据区
//! [99]      校验字节（前 99 字节累加和 % 256）
//! ```
//!
//! ## 帧类型映射
//!
//! | 字节 | 枚举 | 含义 |
//! |---|---|---|
//! | 0xEF | CSSZZL | 参数设置 |
//! | 0xED | CSDQZL | 参数读取 |
//! | 0xDE | SYSJXZZL | 试验数据下载（实时遥测） |
//! | 0xDC | SYSJSK | 试验数据首块（开始 CSV 记录） |
//! | 0xBD | SYSJZJK | 试验数据中间块（写入 CSV） |
//! | 0xDB | SYSJMK | 试验数据末块 |
//! | 0xBF | JBCSQCZL | 基本参数清除 |
//! | 0xBE | SYSJQCZL | 试验数据清除 |
//!
//! ## 关键语法
//!
//! - **`Arc<Mutex<Option<ExtractedFrame>>>`**：线程安全的共享状态。
//!   `Arc` 允许多个线程持有同一份数据的引用计数指针，
//!   `Mutex` 保证互斥访问，`Option` 表示"可能还没有数据"。
//! - **`impl Fn(&[u8]) -> bool + 'static`**：闭包类型签名，
//!   用作帧解码器的回调函数，由 `make_decoder` 工厂函数创建。

use std::sync::Arc;
use std::time::Instant;

/// 帧总长度（字节）
pub const FRAME_LEN: usize = 100;
/// 帧头标识字节（3 字节）
pub const HEADER: [u8; 3] = [0xEB, 0x90, 0x64];

/// 帧类型枚举（对应帧数据中第 4 字节的类型标识）
#[derive(Debug, Clone)]
pub enum FrameType {
    /// 未知/空类型
    NULL,
    /// 参数设置指令（0xEF）
    CSSZZL,
    /// 参数读取指令（0xED）
    CSDQZL,
    /// 试验数据下载/实时遥测（0xDE）
    SYSJXZZL,
    /// 试验数据首块（0xDC），开始 CSV 记录
    SYSJSK,
    /// 试验数据中间块（0xBD），写入 CSV
    SYSJZJK,
    /// 试验数据末块（0xDB），结束 CSV 记录
    SYSJMK,
    /// 基本参数清除（0xBF）
    JBCSQCZL,
    /// 试验数据清除（0xBE）
    SYSJQCZL,
}

/// 提取并校验后的帧数据结构
///
/// 包含帧类型和完整的 100 字节帧数据（原始字节向量）。
#[derive(Debug, Clone)]
pub struct ExtractedFrame {
    /// 帧类型（由第 4 字节决定）
    pub frame_type: FrameType,
    /// 完整的帧数据（100 字节）
    pub data: Vec<u8>,
}

/// 校验帧数据的完整性和正确性
///
/// 检查项：
/// 1. 帧长度 >= 100 字节
/// 2. 帧头为 `[0xEB, 0x90, 0x64]`
/// 3. 校验字节 = 前 99 字节累加和 % 256
pub fn frame_validator(frame: &[u8]) -> bool {
    if frame.len() < FRAME_LEN {
        return false;
    }
    if frame[0] != HEADER[0] || frame[1] != HEADER[1] || frame[2] != HEADER[2] {
        return false;
    }
    // 计算前 99 字节的累加和，取低 8 位与帧尾校验字节比较
    let sum: u16 = frame[..FRAME_LEN - 1]
        .iter()
        .map(|&b| b as u16)
        .sum::<u16>()
        % 256u16;
    (sum as u8) == frame[FRAME_LEN - 1]
}

/// 创建帧解码器闭包
///
/// 返回一个闭包，接收校验通过的帧数据，解析帧类型并存入共享状态。
/// `result` 参数是线程安全的共享容器，解码器闭包写入 `Some(ExtractedFrame)`。
///
/// 闭包类型 `impl Fn(&[u8]) -> bool + 'static` 表示：
/// - `Fn`：不可变借用捕获的变量（可多次调用）
/// - `'static`：闭包不引用非静态生命周期的数据
pub fn make_decoder(
    result: Arc<std::sync::Mutex<Option<ExtractedFrame>>>,
) -> impl Fn(&[u8]) -> bool + 'static {
    move |frame: &[u8]| {
        // 根据帧数据第 4 字节（index=3）解析帧类型
        let frame_type = match frame[3] {
            0xEF => FrameType::CSSZZL,
            0xED => FrameType::CSDQZL,
            0xDE => FrameType::SYSJXZZL,
            0xDC => FrameType::SYSJSK,
            0xBD => FrameType::SYSJZJK,
            0xDB => FrameType::SYSJMK,
            0xBF => FrameType::JBCSQCZL,
            0xBE => FrameType::SYSJQCZL,
            _ => FrameType::NULL,
        };
        let data = frame.to_vec();
        // 将解码结果写入共享状态，供主循环 `try_lock` 取走
        *result.lock().unwrap_or_else(|e| e.into_inner()) = Some(ExtractedFrame { frame_type, data });
        true
    }
}

/// 将字节数组转换为十六进制字符串（如 `[0xEB, 0x90]` → `"EB90"`）
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

/// 解码 100 字节帧为 28 个工程字段（仅使用前 50 字节数据区）
///
/// 各字段的字节偏移和转换公式参照协议文档，主要转换包括：
/// - 有符号 16 位整数：`frame[i] + frame[i+1] * 256`（小端序）
/// - 温度字段：减去 273.0 将开尔文转为摄氏度
/// - 频率/转速：除以 100.0 得到实际值
///
/// 返回 `Vec<String>`，每个元素对应 `CSV_HEADERS` 中的一个字段。
pub fn decode(start_time: Arc<Instant>, frame: &[u8]) -> Vec<String> {
    if frame.len() < 50 {
        return vec![];
    }
    // 计算自服务启动以来的经过时间（秒.毫秒）
    let elapsed = Instant::now().duration_since(*start_time.as_ref());
    let time_str = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_millis());
    // 帧计数（第 5 字节）
    let frame_count = format!("{:.0}", frame[4]);
    // 飞行马赫数（第 6 字节，除以 100）
    let fxmhs_str = format!("{:.2}", frame[5] as f32 / 100.0);
    // 海拔高度（第 7-8 字节，16 位有符号小端序）
    let hbgd_str = format!("{:.0}", (frame[7] as i32 + (frame[6] as i32) * 256) as f32);
    // 燃气发生器转速 Ng（第 9-10 字节）
    let rqfsqzsng_str = format!("{:.2}", (frame[9] as i32 + (frame[8] as i32) * 256) as f32);
    // 排气温度（第 11-12 字节，减去 273 转摄氏度）
    let pqwd_str = format!(
        "{:.2}",
        (frame[11] as i32 + (frame[10] as i32) * 256) as f32 - 273.0
    );
    // 进气温度（第 13-14 字节，乘以 0.1 后减 273）
    let jqwd_str = format!(
        "{:.2}",
        (frame[13] as i32 + (frame[12] as i32) * 256) as f32 * 0.1 - 273.0
    );
    // 动力涡轮转速 Np（第 15-16 字节）
    let dlwlzsnp_str = format!(
        "{:.2}",
        (frame[15] as i32 + (frame[14] as i32) * 256) as f32
    );
    // 油门（第 17-18 字节，乘以 0.01）
    let ym_str = format!(
        "{:.2}",
        (frame[17] as i32 + (frame[16] as i32) * 256) as f32 * 0.01
    );

    // 发动机状态（第 19 字节，枚举映射）
    let fdjzt = match frame[18] {
        0x00 => "空闲",
        0x01 => "待机",
        0x02 => "起动中",
        0x05 => "慢车",
        0x06 => "运行",
        0x07 => "Np 自动闭环",
        0x08 => "停车",
        0x09 => "冷运转",
        0x0A => "油路排气",
        0xA0 => "故障",
        _ => "未知状态",
    };

    // 工作电压（第 20 字节，除以 100）
    let gzdy_str = format!("{:.2}", frame[19] as f32 / 100.0);

    // 控制指令执行情况（第 21 字节，枚举映射）
    let kzzlzxqk = match frame[20] {
        0xA1 => "起动指令执行中",
        0xA2 => "起动成功",
        0xA3 => "起动失败",
        0xB1 => "燃气发生器冷运转指令执行中",
        0xB2 => "燃气发生器冷运转指令执行正常",
        0xB3 => "燃气发生器冷运转指令执行异常",
        0xC1 => "停止燃气发生器冷运转指令执行中",
        0xC2 => "停止燃气发生器冷运转指令执行正常",
        0xC3 => "停止燃气发生器冷运转指令执行异常",
        0xD1 => "油门设定",
        0xE1 => "停车执行中",
        0xE2 => "停车正常",
        0xF1 => "油路排气",
        0x11 => "电控气自检",
        0x00 => "空白指令回复",
        _ => "未知指令",
    };

    // 故障码 1 和 2（第 22-25 字节，十六进制格式）
    let gzm1_str = format!(
        "{:#X}",
        (frame[22] as i32 + (frame[21] as i32) * 256) as u16
    );
    let gzm2_str = format!(
        "{:#X}",
        (frame[24] as i32 + (frame[23] as i32) * 256) as u16
    );

    // 滑油压力（第 26 字节，乘以 0.01）
    let hyyl_str = format!("{:.2}", frame[25] as f32 * 0.01);
    // 滑油温度（第 27-28 字节，乘以 0.1 后减 273）
    let hywd_str = format!(
        "{:.1}",
        (frame[27] as i32 + (frame[26] as i32) * 256) as f32 * 0.1 - 273.0
    );
    // 燃油压力（第 29-30 字节）
    let ryyl_str = format!(
        "{:.0}",
        (frame[29] as i32 + (frame[28] as i32) * 256) as f32
    );

    // 起动电源电压和工作电源电压（预留为 0.0）
    let qddydy_str = format!("{:.2}", 0.0);
    let gzdydy_str = format!("{:.2}", 0.0);

    // 动力涡轮转速 A（第 33-34 字节）
    let dlwlzsa_str = format!(
        "{:.2}",
        (frame[33] as i32 + (frame[32] as i32) * 256) as f32
    );
    // 动力涡轮转速 B（第 35-36 字节）
    let dlwlzsb_str = format!(
        "{:.2}",
        (frame[35] as i32 + (frame[34] as i32) * 256) as f32
    );

    // 发电电压（预留为 0.0）
    let fddy_str = format!("{:.2}", 0.0);

    // 换热器出口滑油温度（第 38-39 字节）
    let hrqckhywd_str = format!(
        "{:.2}",
        (frame[38] as i32 + (frame[37] as i32) * 256) as f32
    );

    // 附件状态（第 38 字节，十六进制）
    let fjzt_str = format!("{:#X}", frame[37]);
    // 燃气发生器反馈故障码及状态码（第 41 字节）
    let rqfsqfkgzmjztm_str = format!("{:.2}", frame[40] as i32);
    // 燃气发生器发送油门（第 42-43 字节，乘以 0.01）
    let rqfsqfsym_str = format!(
        "{:.2}",
        (frame[42] as i32 + (frame[41] as i32) * 256) as f32 * 0.01
    );
    // 指纹码（第 44-48 字节，4 字节十六进制）
    let zwm = format!("0x{}", bytes_to_hex(&frame[43..47]));
    // 燃气发生器故障及状态码（预留为 0.0）
    let rqfsqgzjztm_str = format!("{:.2}", 0.0);

    vec![
        time_str,
        frame_count.to_string(),
        fxmhs_str,
        hbgd_str,
        rqfsqzsng_str,
        pqwd_str,
        jqwd_str,
        dlwlzsnp_str,
        ym_str,
        fdjzt.to_string(),
        gzdy_str,
        kzzlzxqk.to_string(),
        gzm1_str,
        gzm2_str,
        hyyl_str,
        hywd_str,
        ryyl_str,
        qddydy_str,
        gzdydy_str,
        dlwlzsa_str,
        dlwlzsb_str,
        fddy_str,
        hrqckhywd_str,
        fjzt_str,
        rqfsqfkgzmjztm_str,
        rqfsqfsym_str,
        zwm,
        rqfsqgzjztm_str,
    ]
}

/// CSV 记录的 28 列表头（与 decode() 返回顺序一致）
pub const CSV_HEADERS: [&str; 28] = [
    "时间",
    "帧计数",
    "飞行马赫数",
    "海拔高度",
    "燃气发生器转速 Ng",
    "排气温度",
    "进气温度",
    "动力涡轮转速 Np",
    "油门",
    "发动机状态",
    "工作电压",
    "控制指令执行情况",
    "故障码 1",
    "故障码 2",
    "滑油压力",
    "滑油温度",
    "燃油压力",
    "起动电源电压",
    "工作电源电压",
    "动力涡轮转速 A",
    "动力涡轮转速 B",
    "发电电压",
    "换热器出口滑油温度",
    "附件状态",
    "燃气发生器反馈故障码及状态码",
    "燃气发生器发送油门",
    "指纹码",
    "燃气发生器故障及状态码",
];
