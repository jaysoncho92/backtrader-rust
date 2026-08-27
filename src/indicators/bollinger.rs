use std::collections::VecDeque;

use super::Indicator;

/// Bollinger Bands：布林带指标
/// 三条输出线：Middle (SMA), Upper (Middle + dev*StdDev), Lower (Middle - dev*StdDev)
pub struct BollingerBands {
    period: usize,
    dev_factor: f64,
    buffer: VecDeque<f64>,
    sum: f64,
    /// 缓存输出
    middle: f64,
    upper: f64,
    lower: f64,
}

impl BollingerBands {
    /// 创建布林带指标，默认 period=20, dev_factor=2.0
    pub fn new(period: usize, dev_factor: f64) -> Self {
        assert!(period > 0, "布林带周期必须大于 0");
        Self {
            period,
            dev_factor,
            buffer: VecDeque::with_capacity(period),
            sum: 0.0,
            middle: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }

    /// 获取当前布林带三条线 (middle, upper, lower)
    pub fn values(&self) -> Option<(f64, f64, f64)> {
        if self.is_ready() {
            Some((self.middle, self.upper, self.lower))
        } else {
            None
        }
    }

    /// 计算当前窗口的标准差
    fn calc_stddev(&self, mean: f64) -> f64 {
        let n = self.buffer.len() as f64;
        let variance: f64 = self.buffer.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n;
        variance.sqrt()
    }

    /// 内部更新逻辑
    fn update(&mut self, value: f64) -> Option<Vec<f64>> {
        if self.buffer.len() >= self.period {
            let old = self.buffer.pop_front().unwrap();
            self.sum -= old;
        }
        self.buffer.push_back(value);
        self.sum += value;

        if self.buffer.len() < self.period {
            return None;
        }

        self.middle = self.sum / self.period as f64;
        let stddev = self.calc_stddev(self.middle);
        self.upper = self.middle + self.dev_factor * stddev;
        self.lower = self.middle - self.dev_factor * stddev;

        Some(vec![self.middle, self.upper, self.lower])
    }
}

impl Indicator for BollingerBands {
    fn name(&self) -> &str {
        "BollingerBands"
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        self.update(value).map(|v| v[0]) // 返回 middle
    }

    fn output_count(&self) -> usize {
        3
    }

    fn next_multi(&mut self, value: f64) -> Option<Vec<f64>> {
        self.update(value)
    }

    fn min_period(&self) -> usize {
        self.period
    }

    fn is_ready(&self) -> bool {
        self.buffer.len() >= self.period
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.sum = 0.0;
        self.middle = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_basic() {
        let mut bb = BollingerBands::new(3, 2.0);
        assert_eq!(bb.next(1.0), None);
        assert_eq!(bb.next(2.0), None);
        // 第三个值: mean=2.0, stddev=sqrt(((1-2)^2+(2-2)^2+(3-2)^2)/3) = sqrt(2/3)
        let result = bb.next_multi(3.0).unwrap();
        assert_eq!(result.len(), 3);
        let mean = 2.0;
        let stddev = (2.0_f64 / 3.0).sqrt();
        assert!((result[0] - mean).abs() < 1e-9);
        assert!((result[1] - (mean + 2.0 * stddev)).abs() < 1e-9);
        assert!((result[2] - (mean - 2.0 * stddev)).abs() < 1e-9);
    }

    #[test]
    fn test_bollinger_symmetry() {
        // 上下带应关于中线对称
        let mut bb = BollingerBands::new(5, 2.0);
        for v in [10.0, 12.0, 11.0, 13.0, 10.0] {
            bb.next_multi(v);
        }
        let (mid, upper, lower) = bb.values().unwrap();
        assert!((upper - mid - (mid - lower)).abs() < 1e-9);
    }
}
