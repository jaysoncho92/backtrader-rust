use super::Indicator;
use super::ema::EMA;

/// MACD (Moving Average Convergence Divergence)：指数平滑异同移动平均线
/// 三条输出线：MACD线、Signal线、Histogram柱
pub struct MACD {
    /// 快线 EMA
    fast_ema: EMA,
    /// 慢线 EMA
    slow_ema: EMA,
    /// Signal 线 EMA（对 MACD 值做 EMA）
    signal_ema: EMA,
    /// 参数
    #[allow(dead_code)]
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    /// 缓存最新输出
    macd_val: f64,
    signal_val: f64,
    histogram_val: f64,
}

impl MACD {
    /// 创建 MACD 指标，默认参数：fast=12, slow=26, signal=9
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        assert!(fast_period > 0 && slow_period > 0 && signal_period > 0, "MACD 周期必须大于 0");
        assert!(fast_period < slow_period, "快线周期必须小于慢线周期");
        Self {
            fast_ema: EMA::new(fast_period),
            slow_ema: EMA::new(slow_period),
            signal_ema: EMA::new(signal_period),
            fast_period,
            slow_period,
            signal_period,
            macd_val: 0.0,
            signal_val: 0.0,
            histogram_val: 0.0,
        }
    }

    /// 获取 MACD 三条输出线 (macd, signal, histogram)
    pub fn values(&self) -> Option<(f64, f64, f64)> {
        if self.is_ready() {
            Some((self.macd_val, self.signal_val, self.histogram_val))
        } else {
            None
        }
    }

    /// 内部更新逻辑：更新 EMA 并计算三条线
    fn update(&mut self, value: f64) -> Option<Vec<f64>> {
        let _fast_val = self.fast_ema.next(value);
        let slow_val = self.slow_ema.next(value);

        // 慢线 EMA 就绪后才能计算 MACD
        let slow = match slow_val {
            Some(v) => v,
            None => return None,
        };
        // fast_period < slow_period，所以 fast 一定已就绪
        let fast = self.fast_ema.value().unwrap();

        // MACD = EMA(fast) - EMA(slow)
        self.macd_val = fast - slow;

        // Signal = EMA(signal_period) of MACD
        match self.signal_ema.next(self.macd_val) {
            Some(s) => {
                self.signal_val = s;
                self.histogram_val = self.macd_val - self.signal_val;
                Some(vec![self.macd_val, self.signal_val, self.histogram_val])
            }
            None => None,
        }
    }
}

impl Indicator for MACD {
    fn name(&self) -> &str {
        "MACD"
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        self.update(value).map(|v| v[0])
    }

    fn output_count(&self) -> usize {
        3
    }

    fn next_multi(&mut self, value: f64) -> Option<Vec<f64>> {
        self.update(value)
    }

    fn min_period(&self) -> usize {
        // slow_ema 需要 slow_period 个数据点产生第一个值
        // 之后 signal_ema 需要 signal_period 个 MACD 值
        // 总计: slow_period + signal_period - 1
        self.slow_period + self.signal_period - 1
    }

    fn is_ready(&self) -> bool {
        self.fast_ema.is_ready() && self.slow_ema.is_ready() && self.signal_ema.is_ready()
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
        self.macd_val = 0.0;
        self.signal_val = 0.0;
        self.histogram_val = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_output_count() {
        let macd = MACD::new(12, 26, 9);
        assert_eq!(macd.output_count(), 3);
    }

    #[test]
    fn test_macd_min_period() {
        let macd = MACD::new(12, 26, 9);
        assert_eq!(macd.min_period(), 34); // 26 + 9 - 1
    }

    #[test]
    fn test_macd_multi_output() {
        let mut macd = MACD::new(3, 5, 2);
        // min_period = 5 + 2 - 1 = 6
        for i in 1..=5 {
            assert!(macd.next_multi(i as f64).is_none(), "第{}个值应返回None", i);
        }
        // 第6个值应该产生输出
        let result = macd.next_multi(6.0);
        assert!(result.is_some());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 3);
        // histogram = macd - signal
        assert!((lines[2] - (lines[0] - lines[1])).abs() < 1e-9);
    }
}
