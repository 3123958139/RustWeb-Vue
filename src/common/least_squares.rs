//! # 最小二乘法多项式拟合
//!
//! 3 次多项式最小二乘拟合，解正规方程（Gauss 消元法）。
//! 迁移自 fj200c_information Tauri 版 `app/report.rs` 的 `LeastSquareEstimation`。

pub struct LeastSquareEstimation;

impl LeastSquareEstimation {
    pub fn multi_line(arr_x: &[f64], arr_y: &[f64], dimension: usize) -> Vec<f64> {
        let n = dimension + 1;
        let mut guass = vec![vec![0.0_f64; n + 1]; n];

        for i in 0..n {
            for j in 0..n {
                guass[i][j] = Self::sum_arr(arr_x, j + i, arr_x.len());
            }
            guass[i][n] = Self::sum_arr_xy(arr_x, i, arr_y, 1, arr_x.len());
        }

        Self::comput_gauss(&mut guass, n)
    }

    fn sum_arr(arr: &[f64], n: usize, length: usize) -> f64 {
        let mut s = 0.0;
        for i in 0..length {
            if arr[i] != 0.0 || n != 0 {
                s += arr[i].powi(n as i32);
            } else {
                s += 1.0;
            }
        }
        s
    }

    fn sum_arr_xy(arr1: &[f64], n1: usize, arr2: &[f64], n2: usize, length: usize) -> f64 {
        let mut s = 0.0;
        for i in 0..length {
            if (arr1[i] != 0.0 || n1 != 0) && (arr2[i] != 0.0 || n2 != 0) {
                s += arr1[i].powi(n1 as i32) * arr2[i].powi(n2 as i32);
            } else {
                s += 1.0;
            }
        }
        s
    }

    fn comput_gauss(guass: &mut [Vec<f64>], n: usize) -> Vec<f64> {
        let mut x = vec![0.0; n];

        for j in 0..n {
            let mut max = 0.0;
            let mut k = j;
            for i in j..n {
                if guass[i][j].abs() > max {
                    max = guass[i][j].abs();
                    k = i;
                }
            }

            if k != j {
                guass.swap(j, k);
            }

            if max == 0.0 {
                return x;
            }

            for i in (j + 1)..n {
                let s = guass[i][j];
                for m in j..=n {
                    guass[i][m] -= guass[j][m] * s / guass[j][j];
                }
            }
        }

        for i in (0..n).rev() {
            let mut s = 0.0;
            for j in (i + 1)..n {
                s += guass[i][j] * x[j];
            }
            x[i] = (guass[i][n] - s) / guass[i][i];
        }

        x
    }
}
