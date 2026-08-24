//! 数据模型与 CSV 列定义（fj200c_main 模块）
//!
//! 两类内容：
//!
//! - **解码字段结构体**（`EcuFields` / `Adam4015Fields` / `Adam4117Fields` /
//!   `DynoFields` / `FluxFields` / `ChannelData`）：五路串口解码结果的载体，
//!   经 `decode.rs` 填充，序列化后通过 WebSocket 广播给前端；
//!   字段带 `utoipa::ToSchema`，其 `///` 注释会进入 openapi.json 的 schema description
//! - **CSV 列定义**（`csv_entries` / `to_row_values`）：决定 CSV 录制时
//!   每列的顺序、英文键与中文列名，以及取值格式；五路数据拼接后共 64 列
//! - **报表 DTO**（`PerformanceRow` 等）：`report.rs` 报表生成接口的请求/响应类型
use serde::{Deserialize, Serialize};

/// ECU 解码字段（42 字节帧解析结果，见 `decode::decode_ecu`）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct EcuFields {
    /// 燃气发生器转速 Ng（r/min）
    pub ng_speed: f64,
    /// 排气温度（℃）
    pub exhaust_temp: f64,
    /// 动力涡轮转速 Np（r/min）
    pub np_speed: f64,
    /// 飞行马赫数（回传值）
    pub mach_number: f64,
    /// 海拔高度（m）
    pub altitude: f64,
    /// 进气温度（℃）
    pub intake_temp: f64,
    /// 油门开度（%）
    pub throttle: f64,
    /// 发动机状态（中文描述，见 `decode::engine_status_str`）
    pub engine_status: String,
    /// 工作电压（V）
    pub working_voltage: f64,
    /// 控制指令执行情况（中文描述，见 `decode::cmd_exec_str`）
    pub cmd_exec_status: String,
    /// 故障码 1（自检/起动阶段故障位）
    pub fault_code1: u32,
    /// 故障码 2（运行阶段故障位）
    pub fault_code2: u32,
    /// 滑油压力（MPa）
    pub oil_pressure: f64,
    /// 滑油温度（℃）
    pub oil_temp: f64,
    /// 燃油压力
    pub fuel_pressure: f64,
    /// 附件状态（十六进制，低 5 位对应各附件）
    pub accessory_status: String,
    /// 换热器出口滑油温度（℃）
    pub exchanger_outlet_temp: f64,
    /// 指纹码（4 字节十六进制）
    pub fingerprint_code: String,
    /// 帧计数
    pub frame_count: u32,
    /// 停车电磁阀状态
    pub stop_solenoid: bool,
    /// 燃油泵状态
    pub fuel_pump: bool,
    /// 滑油泵状态
    pub oil_pump: bool,
    /// 点火状态（起动中/运行中为 true）
    pub ignition: bool,
    /// 起发电机状态
    pub starter: bool,
    /// 轮载状态
    pub wheel_load_status: bool,
    /// 离心泵
    pub centrifugal_pump: bool,
    /// 故障码按位展开的标志集合
    pub fault_codes: FaultCodeFlags,
    /// 发动机状态原始字节（十六进制）
    pub engine_status_u8: String,
    /// 指令执行状态原始字节（十六进制）
    pub cmd_exec_u8: String,
}

/// 故障码按位展开的标志集合（fc1/fc2 各 16 位，每位对应一个故障）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FaultCodeFlags {
    /// 自检排温异常
    pub fc1_self_check_exhaust: bool,
    /// 自检进温异常
    pub fc1_self_check_intake_temp: bool,
    /// 自检滑压异常
    pub fc1_self_check_oil_pressure: bool,
    /// 自检滑温异常
    pub fc1_self_check_oil_temp: bool,
    /// 自检燃压异常
    pub fc1_self_check_fuel_pressure: bool,
    /// 自检 Ng 转速异常
    pub fc1_self_check_ng_speed: bool,
    /// 自检 Np 转速异常
    pub fc1_self_check_np_speed: bool,
    /// 油路排气异常
    pub fc1_self_check_fuel_vent: bool,
    /// 冷运转异常
    pub fc1_cold_start_abnormal: bool,
    /// 点火失败
    pub fc1_ignition_failure: bool,
    /// 起动超温
    pub fc1_overtemp: bool,
    /// 起动超时
    pub fc1_start_timeout: bool,
    /// 起发转速低
    pub fc1_start_speed_low: bool,
    /// Ng 转速超转
    pub fc1_ng_overspeed: bool,
    /// Np 转速超转
    pub fc1_np_overspeed: bool,
    /// 排温超温
    pub fc1_exhaust_overtemp: bool,
    /// Ng 转速故障
    pub fc2_ng_speed_fault: bool,
    /// Np 转速故障
    pub fc2_np_speed_fault: bool,
    /// 排温故障
    pub fc2_exhaust_temp_fault: bool,
    /// 滑温故障
    pub fc2_oil_temp_fault: bool,
    /// 滑压故障
    pub fc2_oil_pressure_fault: bool,
    /// 燃压故障
    pub fc2_fuel_pressure_fault: bool,
    /// ECU 电压异常
    pub fc2_voltage_abnormal: bool,
    /// 起动电压异常
    pub fc2_start_voltage_abnormal: bool,
    /// 发电电压异常
    pub fc2_gen_voltage_abnormal: bool,
    /// 空中熄火
    pub fc2_in_flight_flameout: bool,
    /// 燃气通信断开
    pub fc2_comm_disconnected_fuel: bool,
    /// 控制通信断开
    pub fc2_comm_disconnected_control: bool,
    /// 油门错误
    pub fc2_throttle_fault: bool,
    /// 起动错误
    pub fc2_start_fault: bool,
}

/// Adam4015 模拟量采集模块解码字段（8 通道电压，V）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct Adam4015Fields {
    /// 8 通道电压值（V）
    pub channels: [f64; 8],
}

/// Adam4117 模拟量采集模块解码字段（8 通道电压，V）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct Adam4117Fields {
    /// 8 通道电压值（V）
    pub channels: [f64; 8],
}

/// 测功机解码字段
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct DynoFields {
    /// 扭矩转速（r/min）
    pub njzs: f64,
    /// 扭矩（N·m）
    pub nj: f64,
    /// 扭矩功率（kW，由 扭矩×转速/9550 计算）
    pub njgl: f64,
}

/// 燃油流量计解码字段
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FluxFields {
    /// 燃油流量
    pub ll: f64,
}

/// 五路串口的解码结果枚举（WebSocket 广播载荷，按 `type` 字段区分数据源）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ChannelData {
    /// ECU 发动机数据
    Ecu(EcuFields),
    /// Adam4015 环境数据
    Adam4015(Adam4015Fields),
    /// Adam4117 环境数据
    Adam4117(Adam4117Fields),
    /// 测功机数据
    Dyno(DynoFields),
    /// 燃油流量计数据
    Flux(FluxFields),
}

/// 试验信息（报表抬头使用，前端录入后保存）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentInfo {
    /// 发动机编号
    pub engine_no: String,
    /// 燃气发生器编号
    pub gas_generator_no: String,
    /// 控制器编号
    pub controller_no: String,
    /// 转速传感器编号
    pub speed_sensor_no: String,
    /// 滑油传感器编号
    pub oil_sensor_no: String,
    /// 试验项目
    pub test_item: String,
    /// 试验时间
    pub test_time: String,
}

/// 布尔值转 CSV 字符串
fn fmt_bool(v: bool) -> String {
    if v {
        "true".into()
    } else {
        "false".into()
    }
}

impl EcuFields {
    /// ECU 的 CSV 列定义：[(英文键, 中文列名)]，顺序即 CSV 列顺序
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[
            ("ngSpeed", "燃气发生器转速Ng"),
            ("exhaustTemp", "排气温度"),
            ("npSpeed", "动力涡轮转速Np"),
            ("machNumber", "飞行马赫数回传"),
            ("altitude", "海拔高度回传"),
            ("intakeTemp", "进气温度"),
            ("throttle", "油门"),
            ("engineStatus", "发动机状态"),
            ("workingVoltage", "工作电压"),
            ("cmdExecStatus", "控制指令执行情况"),
            ("faultCode1", "故障码1"),
            ("faultCode2", "故障码2"),
            ("oilPressure", "滑油压力"),
            ("oilTemp", "滑油温度"),
            ("fuelPressure", "燃油压力"),
            ("accessoryStatus", "附件状态"),
            ("exchangerOutletTemp", "换热器出口滑油温度"),
            ("fingerprintCode", "指纹码"),
            ("frameCount", "帧计数"),
            ("fc1SelfCheckExhaust", "自检排温异常"),
            ("fc1SelfCheckIntakeTemp", "自检进温异常"),
            ("fc1SelfCheckOilPressure", "自检滑压异常"),
            ("fc1SelfCheckOilTemp", "自检滑温异常"),
            ("fc1SelfCheckFuelPressure", "自检燃压异常"),
            ("fc1SelfCheckNgSpeed", "自检Ng转速异常"),
            ("fc1SelfCheckNpSpeed", "自检Np转速异常"),
            ("fc1SelfCheckFuelVent", "油路排气异常"),
            ("fc1ColdStartAbnormal", "冷运转异常"),
            ("fc1IgnitionFailure", "点火失败"),
            ("fc1Overtemp", "起动超温"),
            ("fc1StartTimeout", "起动超时"),
            ("fc1StartSpeedLow", "起发转速低"),
            ("fc1NgOverspeed", "Ng转速超转"),
            ("fc1NpOverspeed", "Np转速超转"),
            ("fc1ExhaustOvertemp", "排温超温"),
            ("fc2NgSpeedFault", "Ng转速故障"),
            ("fc2NpSpeedFault", "Np转速故障"),
            ("fc2ExhaustTempFault", "排温故障"),
            ("fc2OilTempFault", "滑温故障"),
            ("fc2OilPressureFault", "滑压故障"),
            ("fc2FuelPressureFault", "燃压故障"),
            ("fc2VoltageAbnormal", "ECU电压异常"),
            ("fc2StartVoltageAbnormal", "起动电压异常"),
            ("fc2GenVoltageAbnormal", "发电电压异常"),
            ("fc2InFlightFlameout", "空中熄火"),
            ("fc2CommDisconnectedFuel", "燃气通信断开"),
            ("fc2CommDisconnectedControl", "控制通信断开"),
            ("fc2ThrottleFault", "油门设置错误"),
            ("fc2StartFault", "允许起动信号失效"),
            ("stopSolenoid", "停车电磁阀"),
            ("fuelPump", "燃油泵"),
            ("oilPump", "滑油泵"),
            ("starter", "起发电机"),
            ("wheelLoadStatus", "轮载状态"),
            ("centrifugalPump", "离心泵"),
            ("ignition", "点火"),
        ]
    }

    /// 按列定义生成该帧的 CSV 行值（按 csv_entries 顺序取值）
    pub fn to_row_values(&self) -> Vec<String> {
        let fc = &self.fault_codes;
        Self::csv_entries()
            .iter()
            .map(|(key, _)| self.fmt_field(key, fc))
            .collect()
    }

    /// 按列键格式化单个字段值（数值列带小数位数，故障/状态列转字符串）
    fn fmt_field(&self, key: &str, fc: &FaultCodeFlags) -> String {
        match key {
            "ngSpeed" => format!("{:.0}", self.ng_speed),
            "exhaustTemp" => format!("{:.1}", self.exhaust_temp),
            "npSpeed" => format!("{:.0}", self.np_speed),
            "machNumber" => format!("{:.2}", self.mach_number),
            "altitude" => format!("{:.0}", self.altitude),
            "intakeTemp" => format!("{:.1}", self.intake_temp),
            "throttle" => format!("{:.0}", self.throttle),
            "engineStatus" => self.engine_status.clone(),
            "workingVoltage" => format!("{:.1}", self.working_voltage),
            "cmdExecStatus" => self.cmd_exec_status.clone(),
            "faultCode1" => self.fault_code1.to_string(),
            "faultCode2" => self.fault_code2.to_string(),
            "oilPressure" => format!("{:.1}", self.oil_pressure),
            "oilTemp" => format!("{:.1}", self.oil_temp),
            "fuelPressure" => format!("{:.0}", self.fuel_pressure),
            "accessoryStatus" => self.accessory_status.clone(),
            "exchangerOutletTemp" => format!("{:.1}", self.exchanger_outlet_temp),
            "fingerprintCode" => self.fingerprint_code.clone(),
            "frameCount" => self.frame_count.to_string(),
            "fc1SelfCheckExhaust" => fmt_bool(fc.fc1_self_check_exhaust),
            "fc1SelfCheckIntakeTemp" => fmt_bool(fc.fc1_self_check_intake_temp),
            "fc1SelfCheckOilPressure" => fmt_bool(fc.fc1_self_check_oil_pressure),
            "fc1SelfCheckOilTemp" => fmt_bool(fc.fc1_self_check_oil_temp),
            "fc1SelfCheckFuelPressure" => fmt_bool(fc.fc1_self_check_fuel_pressure),
            "fc1SelfCheckNgSpeed" => fmt_bool(fc.fc1_self_check_ng_speed),
            "fc1SelfCheckNpSpeed" => fmt_bool(fc.fc1_self_check_np_speed),
            "fc1SelfCheckFuelVent" => fmt_bool(fc.fc1_self_check_fuel_vent),
            "fc1ColdStartAbnormal" => fmt_bool(fc.fc1_cold_start_abnormal),
            "fc1IgnitionFailure" => fmt_bool(fc.fc1_ignition_failure),
            "fc1Overtemp" => fmt_bool(fc.fc1_overtemp),
            "fc1StartTimeout" => fmt_bool(fc.fc1_start_timeout),
            "fc1StartSpeedLow" => fmt_bool(fc.fc1_start_speed_low),
            "fc1NgOverspeed" => fmt_bool(fc.fc1_ng_overspeed),
            "fc1NpOverspeed" => fmt_bool(fc.fc1_np_overspeed),
            "fc1ExhaustOvertemp" => fmt_bool(fc.fc1_exhaust_overtemp),
            "fc2NgSpeedFault" => fmt_bool(fc.fc2_ng_speed_fault),
            "fc2NpSpeedFault" => fmt_bool(fc.fc2_np_speed_fault),
            "fc2ExhaustTempFault" => fmt_bool(fc.fc2_exhaust_temp_fault),
            "fc2OilTempFault" => fmt_bool(fc.fc2_oil_temp_fault),
            "fc2OilPressureFault" => fmt_bool(fc.fc2_oil_pressure_fault),
            "fc2FuelPressureFault" => fmt_bool(fc.fc2_fuel_pressure_fault),
            "fc2VoltageAbnormal" => fmt_bool(fc.fc2_voltage_abnormal),
            "fc2StartVoltageAbnormal" => fmt_bool(fc.fc2_start_voltage_abnormal),
            "fc2GenVoltageAbnormal" => fmt_bool(fc.fc2_gen_voltage_abnormal),
            "fc2InFlightFlameout" => fmt_bool(fc.fc2_in_flight_flameout),
            "fc2CommDisconnectedFuel" => fmt_bool(fc.fc2_comm_disconnected_fuel),
            "fc2CommDisconnectedControl" => fmt_bool(fc.fc2_comm_disconnected_control),
            "fc2ThrottleFault" => fmt_bool(fc.fc2_throttle_fault),
            "fc2StartFault" => fmt_bool(fc.fc2_start_fault),
            "stopSolenoid" => fmt_bool(self.stop_solenoid),
            "fuelPump" => fmt_bool(self.fuel_pump),
            "oilPump" => fmt_bool(self.oil_pump),
            "starter" => fmt_bool(self.starter),
            "wheelLoadStatus" => fmt_bool(self.wheel_load_status),
            "centrifugalPump" => fmt_bool(self.centrifugal_pump),
            "ignition" => fmt_bool(self.ignition),
            _ => String::new(),
        }
    }
}

impl Adam4015Fields {
    /// Adam4015 的 CSV 列定义（8 通道：大气温度/湿度/压力/进口温度 + 4 扩展通道）
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[
            ("env_ch0", "大气温度(4015)"),
            ("env_ch1", "大气湿度(4015)"),
            ("env_ch2", "大气压力(4015)"),
            ("env_ch3", "进口温度(4015)"),
            ("env_ch4", "扩展通道4(4015)"),
            ("env_ch5", "扩展通道5(4015)"),
            ("env_ch6", "扩展通道6(4015)"),
            ("env_ch7", "扩展通道7(4015)"),
        ]
    }

    pub fn to_row_values(&self) -> Vec<String> {
        self.channels.iter().map(|v| format!("{:.3}", v)).collect()
    }
}

impl Adam4117Fields {
    /// Adam4117 的 CSV 列定义（列名与 4015 对应，标注 (4117) 区分）
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[
            ("env_ch0", "大气温度(4117)"),
            ("env_ch1", "大气湿度(4117)"),
            ("env_ch2", "大气压力(4117)"),
            ("env_ch3", "进口温度(4117)"),
            ("env_ch4", "扩展通道4(4117)"),
            ("env_ch5", "扩展通道5(4117)"),
            ("env_ch6", "扩展通道6(4117)"),
            ("env_ch7", "扩展通道7(4117)"),
        ]
    }

    pub fn to_row_values(&self) -> Vec<String> {
        self.channels.iter().map(|v| format!("{:.3}", v)).collect()
    }
}

impl DynoFields {
    /// 测功机的 CSV 列定义（扭矩转速/扭矩/扭矩功率）
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[("njzs", "扭矩转速"), ("nj", "扭矩"), ("njgl", "扭矩功率")]
    }

    /// 测功机 CSV 行值（转速取整、扭矩 1 位小数、功率 2 位小数）
    pub fn to_row_values(&self) -> Vec<String> {
        vec![
            format!("{:.0}", self.njzs),
            format!("{:.1}", self.nj),
            format!("{:.2}", self.njgl),
        ]
    }
}

impl FluxFields {
    /// 燃油流量计的 CSV 列定义
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[("ll", "燃油流量")]
    }

    /// 燃油流量 CSV 行值（1 位小数）
    pub fn to_row_values(&self) -> Vec<String> {
        vec![format!("{:.1}", self.ll)]
    }
}

/// 汇总五路数据的全部 CSV 列定义（共 64 列，顺序：ECU → 4015 → 4117 → 测功机 → 流量计）
pub fn all_csv_entries() -> Vec<(&'static str, &'static str)> {
    let mut v = Vec::with_capacity(80);
    v.extend_from_slice(EcuFields::csv_entries());
    v.extend_from_slice(Adam4015Fields::csv_entries());
    v.extend_from_slice(Adam4117Fields::csv_entries());
    v.extend_from_slice(DynoFields::csv_entries());
    v.extend_from_slice(FluxFields::csv_entries());
    v
}

/// 汇总五路数据生成一行 CSV 值（与 all_csv_entries 列序一致）
pub fn csv_row_values(
    ecu: &EcuFields,
    adam4015: &Adam4015Fields,
    adam4117: &Adam4117Fields,
    dyno: &DynoFields,
    flux: &FluxFields,
) -> Vec<String> {
    let mut v = Vec::with_capacity(80);
    v.extend(ecu.to_row_values());
    v.extend(adam4015.to_row_values());
    v.extend(adam4117.to_row_values());
    v.extend(dyno.to_row_values());
    v.extend(flux.to_row_values());
    v
}

// ===== 报表 DTO =====

/// 性能数据行（试验报表）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceRow {
    /// 转速
    pub speed: String,
    /// 推力
    pub thrust: String,
    /// 排气温度
    pub exhaust_temp: String,
    /// 燃油流量
    pub fuel_flow: String,
    /// 环境温度
    pub ambient_temp: String,
    /// 环境压力
    pub ambient_pressure: String,
}

/// 标准数据行（与性能数据对比）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StandardRow {
    /// 转速
    pub speed: String,
    /// 推力
    pub thrust: String,
    /// 排气温度
    pub exhaust_temp: String,
    /// 耗油率 SFC
    pub sfc: String,
}

/// 设计点数据行
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesignPointRow {
    /// 转速
    pub speed: String,
    /// 推力
    pub thrust: String,
    /// 排气温度
    pub exhaust_temp: String,
    /// 耗油率 SFC
    pub sfc: String,
}

/// 报表生成结果（试验信息 + 状态点 + 三张数据表）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportOutput {
    /// 试验基本信息（抬头行，见 report.rs 组装逻辑）
    pub basic_info: Vec<String>,
    /// 报表状态点（来自配置 [REPORT] StatePoints）
    pub state_points: Vec<f64>,
    /// 性能数据表
    pub performance_data: Vec<PerformanceRow>,
    /// 标准数据表
    pub standard_data: Vec<StandardRow>,
    /// 设计点数据表
    pub design_point_data: Vec<DesignPointRow>,
}
