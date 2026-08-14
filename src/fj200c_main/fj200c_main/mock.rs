//! 模拟数据源（无需硬件时生成协议帧）
//!
//! 当 `config-fj200c_main.ini` 的 `[MOCK] SimulationMenu = true` 时，
//! 五路串口不再打开真实设备，而是由本模块按各协议格式周期性生成模拟帧：
//!
//! - ECU：42 字节二进制帧（帧头 EB 90 2A + 累加和），参数用正弦波 + 随机噪声模拟
//! - Adam4015/Adam4117：ASCII 帧（`>+ch0+...+ch7\r`，mV 级电压值）
//! - Dyno：18 字节二进制帧（帧头 FF FF + 低 16 位累加和）
//! - Flux：18 字节二进制帧（流量字段 + 累加和）
//!
//! 模拟帧与真实设备帧共用同一校验/解码路径（`decode.rs`），
//! 因此模拟运行可完整验证从前端到解码的整条链路。
use crate::fj200c_main::config;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

/// 模拟数据源类型（决定生成哪种协议的帧）
#[derive(Debug, Clone, PartialEq)]
pub enum MockProfile {
    /// 通用（回落为 ECU 帧）
    Generic,
    /// 发动机电控器（ECU）
    Ecu,
    /// Adam4015 模拟量采集
    Adam,
    /// Adam4117 模拟量采集
    Adam4117,
    /// 测功机
    Dyno,
    /// 燃油流量计
    Flux,
}

impl MockProfile {
    /// 按配置节名推断模拟类型（节名含设备关键字如 ECU/4015/4117/DYNO/FLUX）
    pub fn from_section(section: &str) -> Self {
        let upper = section.to_uppercase();
        if upper.contains("MOCK_COM0") || upper.contains("ECU") {
            MockProfile::Ecu
        } else if upper.contains("MOCK_COM1") || upper.contains("4015") {
            MockProfile::Adam
        } else if upper.contains("MOCK_COM2") || upper.contains("4117") || upper.contains("ADAM") {
            MockProfile::Adam4117
        } else if upper.contains("MOCK_COM3") || upper.contains("DYNO") {
            MockProfile::Dyno
        } else if upper.contains("MOCK_COM4") || upper.contains("FLUX") {
            MockProfile::Flux
        } else {
            MockProfile::Generic
        }
    }
}

/// 模拟控制器：按节配置创建，负责周期性生成该路设备的模拟帧
pub struct MockControl {
    /// 对应配置节名（如 MOCK_COM0）
    section: String,
    /// 模拟类型
    profile: MockProfile,
    /// 帧序号（随帧递增，作为正弦波相位时间基准）
    seq: AtomicU64,
    /// 生成间隔毫秒（取自配置 IntervalMs，默认 100）
    interval_ms: u64,
}

impl MockControl {
    /// 创建模拟控制器（读取配置中的 IntervalMs 与设备类型）
    pub fn create(section: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let interval_ms: u64 = config::global()
            .and_then(|guard| guard.as_ref().map(|c| c.get_or(section, "IntervalMs", "100")))
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let profile = MockProfile::from_section(section);
        info!(
            "[{}] Mock 控制器已创建, profile={:?}, IntervalMs={}",
            section, profile, interval_ms
        );

        Ok(Self {
            section: section.to_string(),
            profile,
            seq: AtomicU64::new(0),
            interval_ms,
        })
    }

    /// 生成间隔毫秒
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// 按 profile 生成对应协议的一帧
    pub fn generate_frame(&self) -> Vec<u8> {
        match self.profile {
            MockProfile::Ecu => self.generate_ecu_frame(),
            MockProfile::Adam | MockProfile::Adam4117 => self.generate_adam_ascii_frame(),
            MockProfile::Dyno => self.generate_dyno_frame(),
            MockProfile::Flux => self.generate_flux_frame(),
            MockProfile::Generic => self.generate_ecu_frame(),
        }
    }

    /// 生成 ECU 帧：关键参数用正弦波模拟趋势，叠加随机噪声，末字节写累加和
    fn generate_ecu_frame(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut f = vec![0u8; 42];
        f[0] = 0xEB;
        f[1] = 0x90;
        f[2] = 0x2A;

        // 帧序号：同时作为正弦波的相位时间基准（t 以 0.1s 递增）
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let fc = seq as f32;
        let t = fc * 0.1;

        // 小端 u16 写入便捷闭包
        let w = |f: &mut [u8], val: u16, off: usize| {
            f[off..off + 2].copy_from_slice(&val.to_le_bytes());
        };

        let frame_cnt = seq as u8;
        f[3] = frame_cnt;
        f[38] = frame_cnt;

        // 马赫数（f[4]）与工作电压（f[5]）
        f[4] = (30.0 + 60.0 * (0.5 + 0.5 * (t * 0.3).sin()) + rng.gen_range(-2.0..2.0)) as u8;
        f[5] = (245 + rng.gen_range(-10..10)) as u8;

        // 高度 / 燃气发生器转速(Ng) / 排气温度 / 进气温度 / 动力涡轮转速(Np) / 油门开度
        w(&mut f, (2000.0 + 8000.0 * (0.5 + 0.5 * (t * 0.1).sin()) + rng.gen_range(-50.0..50.0)) as u16, 6);
        w(&mut f, (5000.0 + 5500.0 * (0.5 + 0.5 * (t * 0.15).sin()) + rng.gen_range(-20.0..20.0)) as u16, 8);
        w(&mut f, (5730.0 + 6000.0 * (0.5 + 0.5 * (t * 0.2).sin()) + rng.gen_range(-30.0..30.0)) as u16, 10);
        w(&mut f, (2730.0 + 500.0 * (0.5 + 0.5 * (t * 0.08).sin()) + rng.gen_range(-10.0..10.0)) as u16, 12);
        w(&mut f, (5000.0 + 6000.0 * (0.5 + 0.5 * (t * 0.18).sin()) + rng.gen_range(-30.0..30.0)) as u16, 14);
        w(&mut f, (40.0 + 50.0 * (0.5 + 0.5 * (t * 0.05).sin()) + rng.gen_range(-3.0..3.0)) as u16, 16);

        // 发动机状态（f[18]）：按周期循环 运行/闭环/起动/慢车 等状态
        let state_cycle = (fc as u64 / 30) % 8;
        f[18] = match state_cycle {
            0 => 0x06,
            1 => 0x07,
            2 => 0x06,
            3 => 0x02,
            4 => 0x06,
            5 => 0x05,
            6 => 0x06,
            _ => 0x06,
        };

        // 指令执行状态（f[19]）：循环展示 起动成功/执行中/油门设定/停车/自检/空白
        let cmd_cycle = (fc as u64 / 40) % 6;
        f[19] = match cmd_cycle {
            0 => 0xA2,
            1 => 0xA1,
            2 => 0xD1,
            3 => 0xE1,
            4 => 0x11,
            _ => 0x00,
        };

        // 故障码：小概率（5%/8%）随机产生，其余时间无故障
        let fc1: u16 = if rng.gen_bool(0.05) { rng.gen_range(1..0x7FFF) } else { 0 };
        w(&mut f, fc1, 20);
        let fc2: u16 = if rng.gen_bool(0.08) { rng.gen_range(1..0x07FF) } else { 0 };
        w(&mut f, fc2, 22);

        // 滑油温度 / 燃油压力
        w(&mut f, (3330 + rng.gen_range(-200..200)) as u16, 24);
        w(&mut f, (150 + rng.gen_range(-30..30)) as u16, 26);

        // 附件状态（f[28]）：循环切换 起动机+滑油泵+燃油泵 / 燃油泵 / 全部关闭
        let acc_cycle = (fc as u64 / 60) % 4;
        f[28] = match acc_cycle {
            0 => 0x07,
            1 => 0x03,
            2 => 0x07,
            _ => 0x00,
        };

        // 滑油压力（f[29]，单位 0.01 MPa）与换热器出口温度
        f[29] = (60 + rng.gen_range(-15..15)) as u8;
        w(&mut f, (3230 + rng.gen_range(-150..150)) as u16, 30);

        // 特征码固定值
        f[34] = 0xAB; f[35] = 0xCD;
        f[36] = 0x12; f[37] = 0x34;

        // 末字节写前 41 字节累加和（与 validate_ecu 校验一致）
        let sum: u16 = f[0..41].iter().map(|&b| b as u16).sum::<u16>();
        f[41] = (sum % 256) as u8;
        f
    }

    /// 生成 Adam ASCII 帧：`>+ch0+...+ch7\r`，前 4 通道正弦模拟，后 4 通道随机
    fn generate_adam_ascii_frame(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) as f32;
        let t = seq * 0.1;

        let ch0 = (25000.0 + 5000.0 * (0.5 + 0.5 * (t * 0.3).sin()) + rng.gen_range(-500.0..500.0)) as i32;
        let ch1 = (45000.0 + 20000.0 * (0.5 + 0.5 * (t * 0.08).sin()) + rng.gen_range(-1000.0..1000.0)) as i32;
        let ch2 = (101325.0 + 3000.0 * (0.5 + 0.5 * (t * 0.1).sin()) + rng.gen_range(-1000.0..1000.0)) as i32;
        let ch3 = (30000.0 + 15000.0 * (0.5 + 0.5 * (t * 0.15).sin()) + rng.gen_range(-500.0..500.0)) as i32;
        let ch4 = rng.gen_range(0..100000);
        let ch5 = rng.gen_range(0..100000);
        let ch6 = rng.gen_range(0..100000);
        let ch7 = rng.gen_range(0..100000);

        let s = format!(
            ">+{:06}+{:06}+{:06}+{:06}+{:06}+{:06}+{:06}+{:06}\r",
            ch0, ch1, ch2, ch3, ch4, ch5, ch6, ch7
        );
        s.into_bytes()
    }

    /// 生成测功机帧：18 字节，转速/扭矩正弦模拟，末 2 字节低 16 位累加和
    fn generate_dyno_frame(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut f = vec![0u8; 18];
        f[0] = 0xFF;
        f[1] = 0xFF;

        let seq = self.seq.fetch_add(1, Ordering::Relaxed) as f32;
        let t = seq * 0.1;

        let jkwd = (250 + rng.gen_range(-10..10)) as u16;
        let njzs = (3000.0 + 2000.0 * (0.5 + 0.5 * (t * 0.15).sin()) + rng.gen_range(-50.0..50.0)) as u16;
        let nj_raw = (500.0 + 300.0 * (0.5 + 0.5 * (t * 0.12).sin()) + rng.gen_range(-10.0..10.0)) as u16;

        f[2..4].copy_from_slice(&jkwd.to_le_bytes());
        f[8..10].copy_from_slice(&njzs.to_le_bytes());
        f[10..12].copy_from_slice(&nj_raw.to_le_bytes());

        let sum: u16 = f[..16].iter().map(|&b| b as u16).sum();
        f[16..18].copy_from_slice(&sum.to_le_bytes());
        f
    }

    /// 生成燃油流量计帧：18 字节，流量字段正弦模拟，末 2 字节累加和
    fn generate_flux_frame(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut f = vec![0u8; 18];
        f[0] = 0xFF;
        f[1] = 0xFF;

        let seq = self.seq.fetch_add(1, Ordering::Relaxed) as f32;
        let t = seq * 0.1;

        let ll = (3000.0 + 1500.0 * (0.5 + 0.5 * (t * 0.2).sin()) + rng.gen_range(-50.0..50.0)) as u16;
        f[2..4].copy_from_slice(&ll.to_le_bytes());

        let sum: u16 = f[..16].iter().map(|&b| b as u16).sum();
        f[16..18].copy_from_slice(&sum.to_le_bytes());
        f
    }
}
