//! 帧校验与解码（fj200c_main 模块）
//!
//! 五路串口各自的上位机/下位机协议帧校验与字段解码：
//!
//! - **ECU**（发动机电控器）：二进制帧，帧头 `EB 90 2A`，42 字节，
//!   末字节为前 41 字节累加和校验；解码出转速/温度/压力/故障码等全部参数
//! - **Adam4015 / Adam4117**（模拟量采集模块）：ASCII 帧，
//!   以 `>` 开头、`\r` 结尾，`+` 分隔 8 通道电压值（单位 mV，除以 1000 转 V）
//! - **Dyno**（测功机）：二进制帧，帧头 `FF FF`，18 字节；解出转速/扭矩/功率
//! - **Flux**（燃油流量计）：二进制帧，帧头 `FF FF`，18 字节；解出流量
//!
//! 每个数据源都提供 `validate_*`（校验帧）与 `decode_*`（解码字段）两个函数，
//! 由 `fj200c_main/fj200c_main/com.rs` 的帧处理线程调用。

use crate::common::utils::format_hex;
use crate::fj200c_main::types::{
    Adam4015Fields, Adam4117Fields, DynoFields, EcuFields, FaultCodeFlags, FluxFields,
};
use tracing::error;

/// ECU 帧头（固定 3 字节）
const ECU_HEADER: [u8; 3] = [0xEB, 0x90, 0x2A];
/// ECU 帧总长度（含末尾校验字节）
const ECU_FRAME_LEN: usize = 42;

/// 校验 ECU 帧：长度、帧头、累加和（前 41 字节之和取低 8 位 == 末字节）
pub fn validate_ecu(frame: &[u8]) -> bool {
    if frame.len() < ECU_FRAME_LEN {
        return false;
    }
    if frame[0] != ECU_HEADER[0] || frame[1] != ECU_HEADER[1] || frame[2] != ECU_HEADER[2] {
        return false;
    }
    let sum: u16 = frame[0..ECU_FRAME_LEN - 1]
        .iter()
        .map(|&b| b as u16)
        .sum::<u16>()
        % 256u16;
    let b = (sum as u8) == frame[ECU_FRAME_LEN - 1];
    if !b {
        tracing::debug!(
            "ECU frame checksum: {}, expected: {}",
            sum,
            frame[ECU_FRAME_LEN - 1]
        );
    }
    b
}

/// 发动机运行状态码 → 中文描述
pub fn engine_status_str(code: u8) -> &'static str {
    match code {
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
    }
}

/// 指令执行状态码 → 中文描述
pub fn cmd_exec_str(code: u8) -> &'static str {
    match code {
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
    }
}

/// 解码 ECU 帧为 `EcuFields`
///
/// 帧内各字段按固定偏移读取（小端 u16），温度类字段需减去 273.0（开尔文转摄氏），
/// 故障码拆分为两位（fc1/fc2）按位展开为 `FaultCodeFlags` 布尔位。
pub fn decode_ecu(frame: &[u8]) -> EcuFields {
    tracing::trace!("{}", &format_hex(frame));
    // 读取小端 u16 的便捷闭包
    let u16_le = |i: usize| -> u16 { (frame[i + 1] as u16) | ((frame[i] as u16) << 8) };

    let _count = frame[3];
    let mach = frame[4] as f64 / 100.0;
    let voltage = frame[5] as f64;
    let altitude = u16_le(6) as f64;
    let ng_speed = u16_le(8) as f64;
    let exhaust_temp = u16_le(10) as f64 - 273.0;
    let intake_temp = u16_le(12) as f64 / 10.0 - 273.0;
    let np_speed = u16_le(14) as f64;
    let throttle_val = u16_le(16) as f64 / 100.0;
    let engine_status_u8 = frame[18];
    let cmd_exec_u8 = frame[19];
    let fc1 = u16_le(20);
    let fc2 = u16_le(22);
    let oil_temp = u16_le(24) as f64 / 10.0 - 273.0;
    let fuel_pressure = u16_le(26) as f64;
    // 附件状态：低 5 位按位表示 停车电磁阀/燃油泵/滑油泵/起动机/轮载
    let accessory = frame[28] & 0x1F;
    let oil_pressure = frame[29] as f64 / 100.0;
    let exchanger_temp = u16_le(30) as f64 / 10.0 - 273.0;
    // 特征码（4 字节，十六进制拼接）
    let fingerprint = frame[34..38]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>();
    let acc_bits = accessory;

    EcuFields {
        ng_speed,
        exhaust_temp,
        np_speed,
        mach_number: mach,
        altitude,
        intake_temp,
        throttle: throttle_val,
        engine_status: engine_status_str(engine_status_u8).to_string(),
        working_voltage: voltage,
        cmd_exec_status: cmd_exec_str(cmd_exec_u8).to_string(),
        fault_code1: fc1 as u32,
        fault_code2: fc2 as u32,
        oil_pressure,
        oil_temp,
        fuel_pressure,
        accessory_status: format!("{:02X}", accessory),
        exchanger_outlet_temp: exchanger_temp,
        fingerprint_code: fingerprint,
        frame_count: frame[38] as u32,
        stop_solenoid: acc_bits & 0x01 != 0,
        fuel_pump: acc_bits & 0x02 != 0,
        oil_pump: acc_bits & 0x04 != 0,
        starter: acc_bits & 0x08 != 0,
        wheel_load_status: acc_bits & 0x10 != 0,
        // 点火状态：起动中或运行中视为点火
        ignition: engine_status_u8 == 0x02 || engine_status_u8 == 0x06,
        fault_codes: FaultCodeFlags {
            fc1_self_check_exhaust: fc1 & (1 << 0) != 0,
            fc1_self_check_intake_temp: fc1 & (1 << 1) != 0,
            fc1_self_check_oil_pressure: fc1 & (1 << 2) != 0,
            fc1_self_check_oil_temp: fc1 & (1 << 3) != 0,
            fc1_self_check_fuel_pressure: fc1 & (1 << 4) != 0,
            fc1_self_check_ng_speed: fc1 & (1 << 5) != 0,
            fc1_self_check_np_speed: fc1 & (1 << 6) != 0,
            fc1_self_check_fuel_vent: fc1 & (1 << 7) != 0,
            fc1_cold_start_abnormal: fc1 & (1 << 8) != 0,
            fc1_ignition_failure: fc1 & (1 << 9) != 0,
            fc1_overtemp: fc1 & (1 << 10) != 0,
            fc1_start_timeout: fc1 & (1 << 11) != 0,
            fc1_start_speed_low: fc1 & (1 << 12) != 0,
            fc1_ng_overspeed: fc1 & (1 << 13) != 0,
            fc1_np_overspeed: fc1 & (1 << 14) != 0,
            fc1_exhaust_overtemp: fc1 & (1 << 15) != 0,
            fc2_ng_speed_fault: fc2 & (1 << 0) != 0,
            fc2_np_speed_fault: fc2 & (1 << 1) != 0,
            fc2_exhaust_temp_fault: fc2 & (1 << 2) != 0,
            fc2_oil_temp_fault: fc2 & (1 << 3) != 0,
            fc2_oil_pressure_fault: fc2 & (1 << 4) != 0,
            fc2_fuel_pressure_fault: fc2 & (1 << 5) != 0,
            fc2_voltage_abnormal: fc2 & (1 << 6) != 0,
            fc2_start_voltage_abnormal: fc2 & (1 << 7) != 0,
            fc2_gen_voltage_abnormal: fc2 & (1 << 8) != 0,
            fc2_in_flight_flameout: fc2 & (1 << 9) != 0,
            fc2_comm_disconnected: fc2 & (1 << 10) != 0,
        },
        engine_status_u8: format!("{:02X}", engine_status_u8),
        cmd_exec_u8: format!("{:02X}", cmd_exec_u8),
    }
}

/// 校验 Adam4015 帧：以 `>` 开头、`\r` 结尾（ASCII 协议）
pub fn validate_adam4015(frame: &[u8]) -> bool {
    if frame.len() < 3 {
        return false;
    }
    if frame[0] != b'>' {
        return false;
    }
    error!("{:?}", frame.iter().map(|x| *x as char).collect::<String>());
    frame[frame.len() - 1] == b'\r'
}

fn parse_adam(input_str: &str) -> Vec<f64> {
    // 目标片段长度
    let chunk_size = 7;

    // 验证输入长度是否为 7 的倍数（可选，防止报错）
    if input_str.len() % chunk_size != 0 {
        eprintln!(
            "警告：输入字符串长度 ({}) 不是 {} 的倍数，尾部数据将被忽略。",
            input_str.len(),
            chunk_size
        );
    }

    // 使用步进切片 (slicing with stride) 来每 7 个字符切分
    // 从索引 0 开始，步长为 7
    let chunks = input_str.chars().collect::<Vec<char>>();

    // 更优雅的方法：直接使用 chars() 迭代，每 7 个一组，或者使用索引步长
    // 这里我们手动构建切片逻辑，确保每个 chunk 是连续的 7 个字符
    let mut floats: Vec<f64> = Vec::new();

    // 计算总共有多少个完整的 chunk
    let total_chars = input_str.len();
    let num_chunks = total_chars / chunk_size;

    for i in 0..num_chunks {
        // 计算当前 chunk 的起始索引
        let start = i * chunk_size;

        // 提取当前 7 个字符
        let chunk = &input_str[start..start + chunk_size];

        // 尝试将字符串转换为 f64
        match chunk.parse::<f64>() {
            Ok(val) => {
                println!("Chunk [{}]: '{}' -> {:.2}", i, chunk, val);
                floats.push(val);
            }
            Err(e) => {
                eprintln!("解析失败: '{}' 无法转换为浮点数，错误: {:?}", chunk, e);
            }
        }
    }

    floats
}

/// 解码 Adam4015 帧：`>ch1+ch2+...+ch8\r`，各通道 mV 值除以 1000 转 V
pub fn decode_adam4015(frame: &[u8]) -> Adam4015Fields {
    let s = std::str::from_utf8(frame).unwrap_or("");
    let mut channels = [0.0f64; 8];

    if s.starts_with('>') && s.ends_with('\r') {
        let inner = s.trim_start_matches('>').trim_end_matches('\r');
        // let parts: Vec<&str> = inner.split('+').filter(|p| !p.is_empty()).collect();
        // for (i, part) in parts.iter().enumerate().take(8) {
        //     let raw: f64 = part.parse().unwrap_or(0.0);
        //     channels[i] = raw / 1000.0;
        // }
        let parsed_data = parse_adam(inner);
        for i in 0..parsed_data.len() {
            error!("i = {:?}", i);
            channels[i] = parsed_data[i];
        }
    }

    Adam4015Fields { channels }
}

/// 校验 Adam4117 帧：格式与 Adam4015 相同（以 `>` 开头、`\r` 结尾）
pub fn validate_adam4117(frame: &[u8]) -> bool {
    if frame.len() < 3 {
        return false;
    }
    if frame[0] != b'>' {
        return false;
    }
    frame[frame.len() - 1] == b'\r'
}

/// 解码 Adam4117 帧：与 Adam4015 相同的 ASCII 格式，8 通道电压
pub fn decode_adam4117(frame: &[u8]) -> Adam4117Fields {
    let s = std::str::from_utf8(frame).unwrap_or("");
    let mut channels = [0.0f64; 8];

    if s.starts_with('>') && s.ends_with('\r') {
        let inner = s.trim_start_matches('>').trim_end_matches('\r');
        // let parts: Vec<&str> = inner.split('+').filter(|p| !p.is_empty()).collect();
        // for (i, part) in parts.iter().enumerate().take(8) {
        //     let raw: f64 = part.parse().unwrap_or(0.0);
        //     channels[i] = raw / 1000.0;
        // }
        let parsed_data = parse_adam(inner);
        for i in 0..parsed_data.len() {
            error!("i = {:?}", i);
            channels[i] = parsed_data[i];
        }
    }

    Adam4117Fields { channels }
}

/// 测功机帧长度
const DYNO_FRAME_LEN: usize = 18;

/// 校验测功机帧：长度 18 字节、帧头 `FF FF`
pub fn validate_dyno(frame: &[u8]) -> bool {
    if frame.len() < DYNO_FRAME_LEN {
        return false;
    }
    if frame[0] != 0xFF || frame[1] != 0xFF {
        return false;
    }
    true
}

/// 解码测功机帧：转速（offset 8）、扭矩（offset 10，除以 10），
/// 功率按 `扭矩 × 转速 / 9550` 计算（转速为 0 时功率为 0）
pub fn decode_dyno(frame: &[u8]) -> DynoFields {
    let u16_le = |i: usize| -> u16 { (frame[i] as u16) | ((frame[i + 1] as u16) << 8) };

    let njzs = u16_le(8) as f64;
    let nj = u16_le(10) as f64 / 10.0;
    let njgl = if njzs > 0.0 { nj * njzs / 9550.0 } else { 0.0 };

    DynoFields { njzs, nj, njgl }
}

/// 燃油流量计帧长度
const FLUX_FRAME_LEN: usize = 18;

/// 校验燃油流量计帧：长度 18 字节、帧头 `FF FF`
pub fn validate_flux(frame: &[u8]) -> bool {
    if frame.len() < FLUX_FRAME_LEN {
        return false;
    }
    if frame[0] != 0xFF || frame[1] != 0xFF {
        return false;
    }
    true
}

/// 解码燃油流量计帧：offset 2 处读取流量（小端 u16）
pub fn decode_flux(frame: &[u8]) -> FluxFields {
    let u16_le = |i: usize| -> u16 { (frame[i] as u16) | ((frame[i + 1] as u16) << 8) };

    let ll = u16_le(2) as f64;

    FluxFields { ll }
}
