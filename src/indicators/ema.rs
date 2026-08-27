use super::Indicator;

/// EMA (Exponential Moving Average)：指数移动平均线
/// 使用平滑因子递推计算，第一个 EMA 值用前 period 个值的 SMA 初始化
pub struct EMA {
    period: usize,
    /// 平滑因子 = 2.0 / (period + 1)
    smoothing: f64,
    /// 当前 EMA 值
    ema: f64,
    /// 已接收的数据计数
    count: usize,
    /// 前 period 个值的累加和（用于 SMA 初始化）
    init_sum: f64,
}

impl EMA {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "EMA 周期必须大于 0");
        Self {
            period,
            smoothing: 2.0 / (period as f64 + 1.0),
            ema: 0.0,
            count: 0,
            init_sum: 0.0,
        }
    }

    /// 获取当前 EMA 值
    pub fn value(&self) -> Option<f64> {
        if self.count >= self.period {
            Some(self.ema)
        } else {
            None
        }
    }
}

impl Indicator for EMA {
    fn name(&self) -> &str {
        "EMA"
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        self.count += 1;

        if self.count < self.period {
            // 尚未达到 period，累加用于 SMA 初始化
            self.init_sum += value;
            return None;
        } else if self.count == self.period {
            // 第 period 个值：用 SMA 作为第一个 EMA 值
            self.init_sum += value;
            self.ema = self.init_sum / self.period as f64;
        } else {
            // 之后：EMA = (value - prev_EMA) * smoothing + prev_EMA
            self.ema = (value - self.ema) * self.smoothing + self.ema;
        }

        Some(self.ema)
    }

    fn min_period(&self) -> usize {
        self.period
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    fn reset(&mut self) {
        self.ema = 0.0;
        self.count = 0;
        self.init_sum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_basic() {
        let mut ema = EMA::new(3);
        // smoothing = 2.0 / (3 + 1) = 0.5
        assert_eq!(ema.next(1.0), None);
        assert_eq!(ema.next(2.0), None);
        // 第三个值：SMA = (1+2+3)/3 = 2.0
        let v = ema.next(3.0).unwrap();
        assert!((v - 2.0).abs() < 1e-9);
        // 第四个值：EMA = (4 - 2.0) * 0.5 + 2.0 = 3.0
        let v = ema.next(4.0).unwrap();
        assert!((v - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = EMA::new(2);
        ema.next(10.0);
        ema.next(20.0);
        assert!(ema.is_ready());
        ema.reset();
        assert!(!ema.is_ready());
    }
}
