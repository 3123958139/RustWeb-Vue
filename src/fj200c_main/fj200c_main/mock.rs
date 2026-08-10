use crate::fj200c_main::config;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum MockProfile {
    Generic,
    Ecu,
    Adam,
    Dyno,
}

impl MockProfile {
    pub fn from_section(section: &str) -> Self {
        let upper = section.to_uppercase();
        if upper.contains("ECU") || upper.contains("MOCK_COM0") {
            MockProfile::Ecu
        } else if upper.contains("ADAM") || upper.contains("MOCK_COM1") {
            MockProfile::Adam
        } else if upper.contains("DYNO") || upper.contains("MOCK_COM2") {
            MockProfile::Dyno
        } else {
            MockProfile::Generic
        }
    }
}

pub struct MockControl {
    section: String,
    profile: MockProfile,
    seq: AtomicU64,
    interval_ms: u64,
}

impl MockControl {
    pub fn create(section: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let interval_ms: u64 = config::global()
            .map(|c| c.get_or(section, "IntervalMs", "100"))
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

    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    pub fn generate_frame(&self) -> Vec<u8> {
        match self.profile {
            MockProfile::Ecu => self.generate_ecu_frame(),
            MockProfile::Adam => self.generate_adam_ascii_frame(),
            MockProfile::Dyno => self.generate_dyno_frame(),
            MockProfile::Generic => self.generate_ecu_frame(),
        }
    }

    fn generate_ecu_frame(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut f = vec![0u8; 42];
        f[0] = 0xEB;
        f[1] = 0x90;
        f[2] = 0x2A;

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let fc = seq as f32;
        let t = fc * 0.1;

        let w = |f: &mut [u8], val: u16, off: usize| {
            f[off..off + 2].copy_from_slice(&val.to_le_bytes());
        };

        let frame_cnt = seq as u8;
        f[3] = frame_cnt;
        f[38] = frame_cnt;

        f[4] = (30.0 + 60.0 * (0.5 + 0.5 * (t * 0.3).sin()) + rng.gen_range(-2.0..2.0)) as u8;
        f[5] = (245 + rng.gen_range(-10..10)) as u8;

        w(&mut f, (2000.0 + 8000.0 * (0.5 + 0.5 * (t * 0.1).sin()) + rng.gen_range(-50.0..50.0)) as u16, 6);
        w(&mut f, (5000.0 + 5500.0 * (0.5 + 0.5 * (t * 0.15).sin()) + rng.gen_range(-20.0..20.0)) as u16, 8);
        w(&mut f, (5730.0 + 6000.0 * (0.5 + 0.5 * (t * 0.2).sin()) + rng.gen_range(-30.0..30.0)) as u16, 10);
        w(&mut f, (2730.0 + 500.0 * (0.5 + 0.5 * (t * 0.08).sin()) + rng.gen_range(-10.0..10.0)) as u16, 12);
        w(&mut f, (5000.0 + 6000.0 * (0.5 + 0.5 * (t * 0.18).sin()) + rng.gen_range(-30.0..30.0)) as u16, 14);
        w(&mut f, (40.0 + 50.0 * (0.5 + 0.5 * (t * 0.05).sin()) + rng.gen_range(-3.0..3.0)) as u16, 16);

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

        let cmd_cycle = (fc as u64 / 40) % 6;
        f[19] = match cmd_cycle {
            0 => 0xA2,
            1 => 0xA1,
            2 => 0xD1,
            3 => 0xE1,
            4 => 0x11,
            _ => 0x00,
        };

        let fc1: u16 = if rng.gen_bool(0.05) { rng.gen_range(1..0x7FFF) } else { 0 };
        w(&mut f, fc1, 20);
        let fc2: u16 = if rng.gen_bool(0.08) { rng.gen_range(1..0x07FF) } else { 0 };
        w(&mut f, fc2, 22);

        w(&mut f, (3330 + rng.gen_range(-200..200)) as u16, 24);
        w(&mut f, (150 + rng.gen_range(-30..30)) as u16, 26);

        let acc_cycle = (fc as u64 / 60) % 4;
        f[28] = match acc_cycle {
            0 => 0x07,
            1 => 0x03,
            2 => 0x07,
            _ => 0x00,
        };

        f[29] = (60 + rng.gen_range(-15..15)) as u8;
        w(&mut f, (3230 + rng.gen_range(-150..150)) as u16, 30);

        f[34] = 0xAB; f[35] = 0xCD;
        f[36] = 0x12; f[37] = 0x34;

        let sum: u16 = f[0..41].iter().map(|&b| b as u16).sum::<u16>();
        f[41] = (sum % 256) as u8;
        f
    }

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
}
