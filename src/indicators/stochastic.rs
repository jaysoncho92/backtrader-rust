use std::collections::VecDeque;

use super::Indicator;
use crate::core::Bar;

/// Stochastic：随机指标（%K 和 %D）
/// %K = (close - lowest_low) / (highest_high - lowest_low) * 100
/// %D = SMA(%K, d_period)
pub struct Stochastic {
    k_period: usize,
    d_period: usize,
    /// 最近 k_period 根 bar 的 high 值
    highs: VecDeque<f64>,
    /// 最近 k_period 根 bar 的 low 值
    lows: VecDeque<f64>,
    /// %K 的滑动窗口（用于计算 %D = SMA(%K)）
    k_buffer: VecDeque<f64>,
    k_sum: f64,
    /// 缓存输出
    k_val: f64,
    d_val: f64,
}

impl Stochastic {
    /// 创建随机指标，默认 k_period=14, d_period=3
    pub fn new(k_period: usize, d_period: usize) -> Self {
        assert!(k_period > 0 && d_period > 0, "Stochastic 周期必须大于 0");
        Self {
            k_period,
            d_period,
            highs: VecDeque::with_capacity(k_period),
            lows: VecDeque::with_capacity(k_period),
            k_buffer: VecDeque::with_capacity(d_period),
            k_sum: 0.0,
            k_val: 0.0,
            d_val: 0.0,
        }
    }

    /// 获取 %K 和 %D
    pub fn values(&self) -> Option<(f64, f64)> {
        if self.is_ready() {
            Some((self.k_val, self.d_val))
        } else {
            None
        }
    }
}

impl Indicator for Stochastic {
    fn name(&self) -> &str {
        "Stochastic"
    }

    fn next(&mut self, _value: f64) -> Option<f64> {
        // Stochastic 需要 OHLC 数据，使用 next_bar
        None
    }

    fn next_bar(&mut self, bar: &Bar) -> Option<f64> {
        // 更新 high/low 窗口
        if self.highs.len() >= self.k_period {
            self.highs.pop_front();
            self.lows.pop_front();
        }
        self.highs.push_back(bar.high);
        self.lows.push_back(bar.low);

        if self.highs.len() < self.k_period {
            return None;
        }

        // 计算 k_period 内的最高价和最低价
        let highest_high = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lowest_low = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);

        // 计算 %K
        let range = highest_high - lowest_low;
        self.k_val = if range == 0.0 {
            50.0 // 高低点相同时 %K = 50
        } else {
            (bar.close - lowest_low) / range * 100.0
        };

        // 更新 %D 窗口（SMA of %K）
        if self.k_buffer.len() >= self.d_period {
            let old_k = self.k_buffer.pop_front().unwrap();
            self.k_sum -= old_k;
        }
        self.k_buffer.push_back(self.k_val);
        self.k_sum += self.k_val;

        if self.k_buffer.len() < self.d_period {
            return None;
        }

        self.d_val = self.k_sum / self.d_period as f64;
        Some(self.k_val)
    }

    fn output_count(&self) -> usize {
        2
    }

    fn next_bar_multi(&mut self, bar: &Bar) -> Option<Vec<f64>> {
        self.next_bar(bar).map(|_| vec![self.k_val, self.d_val])
    }

    fn min_period(&self) -> usize {
        // k_period 根 bar 产生第一个 %K，再需要 d_period-1 个额外 %K 才能产生 %D
        self.k_period + self.d_period - 1
    }

    fn is_ready(&self) -> bool {
        self.highs.len() >= self.k_period && self.k_buffer.len() >= self.d_period
    }

    fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.k_buffer.clear();
        self.k_sum = 0.0;
        self.k_val = 0.0;
        self.d_val = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bar(o: f64, h: f64, l: f64, c: f64) -> Bar {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        Bar::new(dt, o, h, l, c, 1000.0, 0.0)
    }

    #[test]
    fn test_stochastic_range() {
        // %K 和 %D 应在 0-100 范围内
        let mut stoch = Stochastic::new(3, 2);
        let bars = vec![
            make_bar(10.0, 12.0, 8.0, 11.0),
            make_bar(11.0, 13.0, 9.0, 10.0),
            make_bar(10.0, 14.0, 7.0, 13.0),
            make_bar(13.0, 15.0, 10.0, 12.0),
        ];
        for bar in &bars {
            let result = stoch.next_bar_multi(bar);
            if let Some(vals) = result {
                assert!(vals[0] >= 0.0 && vals[0] <= 100.0, "%K 应在 0-100: {}", vals[0]);
                assert!(vals[1] >= 0.0 && vals[1] <= 100.0, "%D 应在 0-100: {}", vals[1]);
            }
        }
    }

    #[test]
    fn test_stochastic_min_period() {
        let stoch = Stochastic::new(14, 3);
        assert_eq!(stoch.min_period(), 16); // 14 + 3 - 1
    }
}
