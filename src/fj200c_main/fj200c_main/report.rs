//! 试验报表生成（从 CSV 录制数据汇总出性能/标准/设计点三张表）
//!
//! 流程：读取录制 CSV（由 `service.rs` 传入）与试验信息 CSV
//! （`<原文件名>-information.csv`，见 `common/utils.rs::rename_file_name`），
//! 按配置的状态点（`[REPORT] StatePoints`）为每个状态点做窗口滤波取平均，
//! 再折算标准大气参数（`ncor`/`fcor`/`sfc_cor`），最后用三次多项式
//! 最小二乘拟合（`common/least_squares.rs`）得到设计点数据。
//!
//! 列来源约定（与 `types.rs` 的 CSV 列名对应）：
//! - 推力 = 测功机「扭矩」列（试车台无独立推力传感器时以扭矩代理）
//! - 燃油流量 = Flux「燃油流量」列
use crate::common::global_var::GlobalVar;
use crate::common::least_squares::LeastSquareEstimation;
use crate::common::utils::{read_csv_to_map, rename_file_name};
use crate::fj200c_main::types::{DesignPointRow, PerformanceRow, ReportOutput, StandardRow};
use tracing::error;

/// 转速折算到标准大气（ISA，15℃）：`N_cor = N × √(288.15 / T)`
fn ncor(n: f64, t: f64) -> f64 {
    if t + 273.15 <= 0.0 {
        return 0.0;
    }
    let r = n * (288.15 / (t + 273.15)).sqrt();
    if r.is_nan() {
        0.0
    } else {
        r
    }
}

/// 推力/燃油流量折算到标准大气压：`F_cor = F × 101.325 / P`
fn fcor(f: f64, p: f64) -> f64 {
    if p == 0.0 {
        return 0.0;
    }
    let r = f * 101.325 / p;
    if r.is_nan() {
        0.0
    } else {
        r
    }
}

/// 耗油率 SFC 折算：`SFC = 3.6 × Qv × √(288.15/T) / F`（Qv 燃油流量，F 推力）
fn sfc_cor(qvf: f64, f: f64, t: f64) -> f64 {
    if f == 0.0 || t + 273.15 <= 0.0 {
        return 0.0;
    }
    let r = 3.6 * qvf * (288.15 / (t + 273.15)).sqrt() / f;
    if r.is_nan() {
        0.0
    } else {
        r
    }
}

/// 前向填充：0 值用前一个非 0 值补齐（处理采样缺失）
fn fill_forward(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return data.to_vec();
    }
    let mut result = data.to_vec();
    for i in 1..data.len() {
        if result[i] == 0.0 && result[i - 1] != 0.0 {
            result[i] = result[i - 1];
        }
    }
    result
}

/// 后向填充：0 值用后一个非 0 值补齐（与前向填充配合覆盖整段）
fn fill_backward(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return data.to_vec();
    }
    let mut result = data.to_vec();
    for i in (0..data.len() - 1).rev() {
        if result[i] == 0.0 && result[i + 1] != 0.0 {
            result[i] = result[i + 1];
        }
    }
    result
}

/// 录制 CSV 中报表所需列的下标（按中文列名定位，列不存在时为 None）
struct CsvColumnIdx {
    /// 燃气发生器转速 Ng
    col_clzs: Option<usize>,
    /// 排气温度
    col_pqwd: Option<usize>,
    /// 扭矩（推力代理列）
    col_tl: Option<usize>,
    /// 燃油流量
    col_ryll: Option<usize>,
    /// 大气温度（Adam4015 通道 0）
    col_dqwd: Option<usize>,
    /// 大气压力（Adam4015 通道 2）
    col_dqyl: Option<usize>,
}

/// 按表头定位各列下标
fn build_col_idx(headers: &[String]) -> CsvColumnIdx {
    let find = |name: &str| headers.iter().position(|h| h == name);
    CsvColumnIdx {
        col_clzs: find("燃气发生器转速Ng"),
        col_pqwd: find("排气温度"),
        // 推力来源：测功机扭矩（试车台无独立推力传感器时以扭矩列代理）
        col_tl: find("扭矩"),
        // 燃油流量来源：Flux 燃油流量计
        col_ryll: find("燃油流量"),
        col_dqwd: find("大气温度(4015)"),
        col_dqyl: find("大气压力(4015)"),
    }
}

/// 字符串转 f64（解析失败按 0 处理）
fn parse_f64(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

/// 由录制 CSV 生成试验报表数据
///
/// # 参数
/// - `file_name`：录制 CSV 文件名（同目录下读取 `<名>-information.csv` 作试验信息）
/// - `content`：录制 CSV 内容（表头 + 数据行）
/// - `state_points`：报表状态点（逗号分隔的转速值，取自配置 `[REPORT] StatePoints`）
///
/// # 返回值
/// - `Ok(ReportOutput)`：试验信息 + 状态点 + 性能/标准/设计点三张表
/// - `Err`：状态点配置为空或 CSV 解析失败
pub fn process_report_csv(
    file_name: &str,
    content: &str,
    state_points: &str,
) -> Result<ReportOutput, String> {
    let file_name_information = rename_file_name(file_name, "information");
    let csv_information_dict = read_csv_to_map(&file_name_information);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("读取CSV表头失败: {}", e))?
        .iter()
        .map(|s| s.to_string())
        .collect();

    let idx = build_col_idx(&headers);

    // 只保留报表需要的列，压缩为 16 元素定长数组：
    // [0]=时间戳 [1]=Ng [2]=排气温度 [7]=扭矩 [8]=燃油流量 [9]=大气压力 [10]=大气温度
    let mut rows_16: Vec<[f64; 16]> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("读取CSV行失败: {}", e))?;
        let get = |col: Option<usize>| -> f64 {
            col.and_then(|i| record.get(i))
                .map(parse_f64)
                .unwrap_or(0.0)
        };
        let ts = record.get(0).map(parse_f64).unwrap_or(0.0);

        let mut r = [0.0_f64; 16];
        r[0] = ts;
        r[1] = get(idx.col_clzs);
        r[2] = get(idx.col_pqwd);
        r[7] = get(idx.col_tl);
        r[8] = get(idx.col_ryll);
        r[9] = get(idx.col_dqyl);
        r[10] = get(idx.col_dqwd);
        rows_16.push(r);
    }

    // 试验信息：优先读 information CSV，缺失时回落全局变量/默认值
    let gv = GlobalVar::global();
    let get_info = |key: &str, default: &str| -> String {
        gv.map(|g| g.get_or(key, default))
            .unwrap_or(default.to_string())
    };

    let basic_info: Vec<String> = match csv_information_dict {
        Ok(info) => {
            vec![
                get_info("engine_no", info.get("发动机编号").map(|s| s.as_str()).unwrap_or("")),
                get_info("gas_generator_no", info.get("燃气发生器编号").map(|s| s.as_str()).unwrap_or("")),
                get_info("controller_no", info.get("电控器编号").map(|s| s.as_str()).unwrap_or("")),
                get_info("speed_sensor_no", info.get("转速传感器编号").map(|s| s.as_str()).unwrap_or("")),
                get_info("oil_sensor_no", info.get("滑油温压一体传感器编号").map(|s| s.as_str()).unwrap_or("")),
                get_info("test_item", info.get("试验项目").map(|s| s.as_str()).unwrap_or("")),
                get_info("test_time", info.get("试验时间").map(|s| s.as_str()).unwrap_or("")),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]
        }
        Err(e) => {
            error!("读取试验信息 CSV 失败: {}", e);
            vec![
                get_info("engine_no", ""),
                get_info("gas_generator_no", ""),
                get_info("controller_no", ""),
                get_info("speed_sensor_no", ""),
                get_info("oil_sensor_no", ""),
                get_info("test_item", ""),
                get_info("test_time", ""),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]
        }
    };

    // 状态点：逗号分隔字符串 → f64 数组
    let points: Vec<f64> = state_points
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();

    if points.is_empty() {
        return Err("状态点配置为空".to_string());
    }

    let n = points.len();
    // 性能表（实测滤波均值）中间结果
    let mut perf_speed = vec![0.0_f64; n];
    let mut perf_thrust = vec![0.0_f64; n];
    let mut perf_exhaust = vec![0.0_f64; n];
    let mut perf_fuel = vec![0.0_f64; n];
    let mut perf_amb_temp = vec![0.0_f64; n];
    let mut perf_amb_press = vec![0.0_f64; n];

    // 标准表（折算后）中间结果
    let mut std_speed = vec![0.0_f64; n];
    let mut std_thrust = vec![0.0_f64; n];
    let mut std_exhaust = vec![0.0_f64; n];
    let mut std_sfc = vec![0.0_f64; n];

    // 设计点表（拟合后）中间结果
    let mut dp_zs = vec![0.0_f64; n];
    let mut dp_tl = vec![0.0_f64; n];
    let mut dp_pqwd = vec![0.0_f64; n];
    let mut dp_hyl = vec![0.0_f64; n];

    for j in 0..n {
        let point = points[j];
        // 状态点 ± 100 r/min 窗口滤波
        let bias = 100.0;

        let filtered: Vec<&[f64; 16]> = rows_16
            .iter()
            .filter(|r| r[1] >= point - bias && r[1] <= point + bias)
            .collect();

        if filtered.is_empty() {
            continue;
        }

        // 窗口内逐列求平均
        let count = filtered.len() as f64;
        let mut sum = [0.0_f64; 16];
        for row in &filtered {
            for k in 0..16 {
                sum[k] += row[k];
            }
        }

        let avg = |idx: usize| sum[idx] / count;

        let clzs = avg(1);
        let pqwd = avg(2);
        let tl = avg(7);
        let ryll = avg(8);
        let dqwd = avg(10);
        let dqyl = avg(9);

        // 性能表：实测值
        perf_speed[j] = clzs;
        perf_thrust[j] = tl;
        perf_exhaust[j] = pqwd;
        perf_fuel[j] = ryll;
        perf_amb_temp[j] = dqwd;
        perf_amb_press[j] = dqyl;

        // 标准表：折算到标准大气（扭矩 10 倍缩放还原为工程值）
        let n_cor = ncor(clzs, dqwd);
        let f_cor = if tl != 0.0 {
            fcor(tl / 10.0, dqyl)
        } else {
            0.0
        };
        let sfc = if tl != 0.0 {
            sfc_cor(ryll, tl / 10.0, dqwd)
        } else {
            0.0
        };

        std_speed[j] = n_cor;
        std_thrust[j] = f_cor;
        std_exhaust[j] = pqwd;
        std_sfc[j] = sfc;

        // 设计点：以标准折算值作为拟合样本
        dp_zs[j] = n_cor;
        dp_tl[j] = f_cor;
        dp_pqwd[j] = pqwd;
        dp_hyl[j] = sfc;
    }

    // 填充缺失状态点（前向+后向），保证拟合样本连续
    let filled_zs = fill_backward(&fill_forward(&dp_zs));
    let filled_tl = fill_backward(&fill_forward(&dp_tl));
    let filled_pqwd = fill_backward(&fill_forward(&dp_pqwd));
    let filled_hyl = fill_backward(&fill_forward(&dp_hyl));

    // 对转速（x）分别拟合 推力/排气温度/耗油率 的三次多项式
    let coeffs_tl = LeastSquareEstimation::multi_line(&filled_zs, &filled_tl, 3);
    let coeffs_pqwd = LeastSquareEstimation::multi_line(&filled_zs, &filled_pqwd, 3);
    let coeffs_hyl = LeastSquareEstimation::multi_line(&filled_zs, &filled_hyl, 3);

    // 三次多项式求值（coeff 按升幂排列）
    let poly = |coeff: &[f64], x: f64| -> f64 {
        if coeff.len() < 4 {
            return 0.0;
        }
        coeff[0] + coeff[1] * x + coeff[2] * x * x + coeff[3] * x * x * x
    };

    // 设计点表：在标准状态点上求拟合值
    let mut dp_result_tl = Vec::with_capacity(n);
    let mut dp_result_pqwd = Vec::with_capacity(n);
    let mut dp_result_hyl = Vec::with_capacity(n);

    for j in 0..n {
        let x = points[j];
        dp_result_tl.push(poly(&coeffs_tl, x));
        dp_result_pqwd.push(poly(&coeffs_pqwd, x));
        dp_result_hyl.push(poly(&coeffs_hyl, x));
    }

    let performance_data: Vec<PerformanceRow> = (0..n)
        .map(|j| PerformanceRow {
            speed: format!("{:.1}", perf_speed[j]),
            thrust: format!("{:.1}", perf_thrust[j]),
            exhaust_temp: format!("{:.1}", perf_exhaust[j]),
            fuel_flow: format!("{:.1}", perf_fuel[j]),
            ambient_temp: format!("{:.1}", perf_amb_temp[j]),
            ambient_pressure: format!("{:.1}", perf_amb_press[j]),
        })
        .collect();

    let standard_data: Vec<StandardRow> = (0..n)
        .map(|j| StandardRow {
            speed: format!("{:.1}", std_speed[j]),
            thrust: format!("{:.3}", std_thrust[j]),
            exhaust_temp: format!("{:.1}", std_exhaust[j]),
            sfc: format!("{:.3}", std_sfc[j]),
        })
        .collect();

    let design_point_data: Vec<DesignPointRow> = (0..n)
        .map(|j| DesignPointRow {
            speed: format!("{:.1}", points[j]),
            thrust: format!("{:.3}", dp_result_tl[j]),
            exhaust_temp: format!("{:.3}", dp_result_pqwd[j]),
            sfc: format!("{:.3}", dp_result_hyl[j]),
        })
        .collect();

    Ok(ReportOutput {
        basic_info,
        state_points: points,
        performance_data,
        standard_data,
        design_point_data,
    })
}
