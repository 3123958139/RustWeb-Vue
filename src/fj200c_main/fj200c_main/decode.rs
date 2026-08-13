use crate::common::utils::format_hex;
use crate::fj200c_main::types::{
    Adam4015Fields, Adam4117Fields, DynoFields, EcuFields, FaultCodeFlags, FluxFields,
};
use tracing::error;

const ECU_HEADER: [u8; 3] = [0xEB, 0x90, 0x2A];
const ECU_FRAME_LEN: usize = 42;

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
        error!(
            "ECU frame checksum: {}, expected: {}",
            sum,
            frame[ECU_FRAME_LEN - 1]
        );
    }
    b
}

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

pub fn decode_ecu(frame: &[u8]) -> EcuFields {
    error!("{}", &format_hex(frame));
    let u16_le = |i: usize| -> u16 { (frame[i] as u16) | ((frame[i + 1] as u16) << 8) };

    let _count = frame[3];
    let mach = frame[4] as f64 / 100.0;
    let voltage = frame[5] as f64 / 10.0;
    let altitude = u16_le(6) as f64;
    let ng_speed = u16_le(8) as f64;
    let exhaust_temp = u16_le(10) as f64 / 10.0;
    let intake_temp = u16_le(12) as f64 / 10.0 - 273.0;
    let np_speed = u16_le(14) as f64;
    let throttle_val = u16_le(16) as f64;
    let engine_status_u8 = frame[18];
    let cmd_exec_u8 = frame[19];
    let fc1 = u16_le(20);
    let fc2 = u16_le(22);
    let oil_temp = u16_le(24) as f64 / 10.0 - 273.0;
    let fuel_pressure = u16_le(26) as f64;
    let accessory = frame[28] & 0x1F;
    let oil_pressure = frame[29] as f64 / 100.0;
    let exchanger_temp = u16_le(30) as f64 / 10.0;
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

pub fn validate_adam4015(frame: &[u8]) -> bool {
    if frame.len() < 3 {
        return false;
    }
    if frame[0] != b'>' {
        return false;
    }
    frame[frame.len() - 1] == b'\r'
}

pub fn decode_adam4015(frame: &[u8]) -> Adam4015Fields {
    let s = std::str::from_utf8(frame).unwrap_or("");
    let mut channels = [0.0f64; 8];

    if s.starts_with('>') && s.ends_with('\r') {
        let inner = s.trim_start_matches('>').trim_end_matches('\r');
        let parts: Vec<&str> = inner.split('+').filter(|p| !p.is_empty()).collect();
        for (i, part) in parts.iter().enumerate().take(8) {
            let raw: f64 = part.parse().unwrap_or(0.0);
            channels[i] = raw / 1000.0;
        }
    }

    Adam4015Fields { channels }
}

pub fn validate_adam4117(frame: &[u8]) -> bool {
    if frame.len() < 3 {
        return false;
    }
    if frame[0] != b'>' {
        return false;
    }
    frame[frame.len() - 1] == b'\r'
}

pub fn decode_adam4117(frame: &[u8]) -> Adam4117Fields {
    let s = std::str::from_utf8(frame).unwrap_or("");
    let mut channels = [0.0f64; 8];

    if s.starts_with('>') && s.ends_with('\r') {
        let inner = s.trim_start_matches('>').trim_end_matches('\r');
        let parts: Vec<&str> = inner.split('+').filter(|p| !p.is_empty()).collect();
        for (i, part) in parts.iter().enumerate().take(8) {
            let raw: f64 = part.parse().unwrap_or(0.0);
            channels[i] = raw / 1000.0;
        }
    }

    Adam4117Fields { channels }
}

const DYNO_FRAME_LEN: usize = 18;

pub fn validate_dyno(frame: &[u8]) -> bool {
    if frame.len() < DYNO_FRAME_LEN {
        return false;
    }
    if frame[0] != 0xFF || frame[1] != 0xFF {
        return false;
    }
    true
}

pub fn decode_dyno(frame: &[u8]) -> DynoFields {
    let u16_le = |i: usize| -> u16 { (frame[i] as u16) | ((frame[i + 1] as u16) << 8) };

    let njzs = u16_le(8) as f64;
    let nj = u16_le(10) as f64 / 10.0;
    let njgl = if njzs > 0.0 { nj * njzs / 9550.0 } else { 0.0 };

    DynoFields { njzs, nj, njgl }
}

const FLUX_FRAME_LEN: usize = 18;

pub fn validate_flux(frame: &[u8]) -> bool {
    if frame.len() < FLUX_FRAME_LEN {
        return false;
    }
    if frame[0] != 0xFF || frame[1] != 0xFF {
        return false;
    }
    true
}

pub fn decode_flux(frame: &[u8]) -> FluxFields {
    let u16_le = |i: usize| -> u16 { (frame[i] as u16) | ((frame[i + 1] as u16) << 8) };

    let ll = u16_le(2) as f64;

    FluxFields { ll }
}
