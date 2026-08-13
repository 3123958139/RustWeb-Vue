use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct EcuFields {
    pub ng_speed: f64,
    pub exhaust_temp: f64,
    pub np_speed: f64,
    pub mach_number: f64,
    pub altitude: f64,
    pub intake_temp: f64,
    pub throttle: f64,
    pub engine_status: String,
    pub working_voltage: f64,
    pub cmd_exec_status: String,
    pub fault_code1: u32,
    pub fault_code2: u32,
    pub oil_pressure: f64,
    pub oil_temp: f64,
    pub fuel_pressure: f64,
    pub accessory_status: String,
    pub exchanger_outlet_temp: f64,
    pub fingerprint_code: String,
    pub frame_count: u32,
    pub stop_solenoid: bool,
    pub fuel_pump: bool,
    pub oil_pump: bool,
    pub ignition: bool,
    pub starter: bool,
    pub wheel_load_status: bool,
    pub fault_codes: FaultCodeFlags,
    pub engine_status_u8: String,
    pub cmd_exec_u8: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FaultCodeFlags {
    pub fc1_self_check_exhaust: bool,
    pub fc1_self_check_intake_temp: bool,
    pub fc1_self_check_oil_pressure: bool,
    pub fc1_self_check_oil_temp: bool,
    pub fc1_self_check_fuel_pressure: bool,
    pub fc1_self_check_ng_speed: bool,
    pub fc1_self_check_np_speed: bool,
    pub fc1_self_check_fuel_vent: bool,
    pub fc1_cold_start_abnormal: bool,
    pub fc1_ignition_failure: bool,
    pub fc1_overtemp: bool,
    pub fc1_start_timeout: bool,
    pub fc1_start_speed_low: bool,
    pub fc1_ng_overspeed: bool,
    pub fc1_np_overspeed: bool,
    pub fc1_exhaust_overtemp: bool,
    pub fc2_ng_speed_fault: bool,
    pub fc2_np_speed_fault: bool,
    pub fc2_exhaust_temp_fault: bool,
    pub fc2_oil_temp_fault: bool,
    pub fc2_oil_pressure_fault: bool,
    pub fc2_fuel_pressure_fault: bool,
    pub fc2_voltage_abnormal: bool,
    pub fc2_start_voltage_abnormal: bool,
    pub fc2_gen_voltage_abnormal: bool,
    pub fc2_in_flight_flameout: bool,
    pub fc2_comm_disconnected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct Adam4015Fields {
    pub channels: [f64; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct Adam4117Fields {
    pub channels: [f64; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct DynoFields {
    pub njzs: f64,
    pub nj: f64,
    pub njgl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FluxFields {
    pub ll: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ChannelData {
    Ecu(EcuFields),
    Adam4015(Adam4015Fields),
    Adam4117(Adam4117Fields),
    Dyno(DynoFields),
    Flux(FluxFields),
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentInfo {
    pub engine_no: String,
    pub gas_generator_no: String,
    pub controller_no: String,
    pub speed_sensor_no: String,
    pub oil_sensor_no: String,
    pub test_item: String,
    pub test_time: String,
}

fn fmt_bool(v: bool) -> String {
    if v {
        "true".into()
    } else {
        "false".into()
    }
}

impl EcuFields {
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
            ("fc2CommDisconnected", "通信断开"),
            ("stopSolenoid", "停车电磁阀"),
            ("fuelPump", "燃油泵"),
            ("oilPump", "滑油泵"),
            ("starter", "起发电机"),
            ("wheelLoadStatus", "轮载状态"),
            ("ignition", "点火"),
        ]
    }

    pub fn to_row_values(&self) -> Vec<String> {
        let fc = &self.fault_codes;
        Self::csv_entries()
            .iter()
            .map(|(key, _)| self.fmt_field(key, fc))
            .collect()
    }

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
            "fc2CommDisconnected" => fmt_bool(fc.fc2_comm_disconnected),
            "stopSolenoid" => fmt_bool(self.stop_solenoid),
            "fuelPump" => fmt_bool(self.fuel_pump),
            "oilPump" => fmt_bool(self.oil_pump),
            "starter" => fmt_bool(self.starter),
            "wheelLoadStatus" => fmt_bool(self.wheel_load_status),
            "ignition" => fmt_bool(self.ignition),
            _ => String::new(),
        }
    }
}

impl Adam4015Fields {
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
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[("njzs", "扭矩转速"), ("nj", "扭矩"), ("njgl", "扭矩功率")]
    }

    pub fn to_row_values(&self) -> Vec<String> {
        vec![
            format!("{:.0}", self.njzs),
            format!("{:.1}", self.nj),
            format!("{:.2}", self.njgl),
        ]
    }
}

impl FluxFields {
    pub fn csv_entries() -> &'static [(&'static str, &'static str)] {
        &[("ll", "燃油流量")]
    }

    pub fn to_row_values(&self) -> Vec<String> {
        vec![format!("{:.1}", self.ll)]
    }
}

pub fn all_csv_entries() -> Vec<(&'static str, &'static str)> {
    let mut v = Vec::with_capacity(80);
    v.extend_from_slice(EcuFields::csv_entries());
    v.extend_from_slice(Adam4015Fields::csv_entries());
    v.extend_from_slice(Adam4117Fields::csv_entries());
    v.extend_from_slice(DynoFields::csv_entries());
    v.extend_from_slice(FluxFields::csv_entries());
    v
}

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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceRow {
    pub speed: String,
    pub thrust: String,
    pub exhaust_temp: String,
    pub fuel_flow: String,
    pub ambient_temp: String,
    pub ambient_pressure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StandardRow {
    pub speed: String,
    pub thrust: String,
    pub exhaust_temp: String,
    pub sfc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesignPointRow {
    pub speed: String,
    pub thrust: String,
    pub exhaust_temp: String,
    pub sfc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportOutput {
    pub basic_info: Vec<String>,
    pub state_points: Vec<f64>,
    pub performance_data: Vec<PerformanceRow>,
    pub standard_data: Vec<StandardRow>,
    pub design_point_data: Vec<DesignPointRow>,
}
