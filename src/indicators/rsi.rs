use super::Indicator;

/// RSI (Relative Strength Index)：相对强弱指数
/// 使用 Wilder 平滑法计算平均涨幅和平均跌幅
pub struct RSI {
    period: usize,
    /// 前一个收盘价（用于计算变化量）
    prev_close: Option<f64>,
    /// 已接收的数据计数
    count: usize,
    /// 平均涨幅
    avg_gain: f64,
    /// 平均跌幅
    avg_loss: f64,
    /// 初始阶段的涨幅累加
    gain_sum: f64,
    /// 初始阶段的跌幅累加
    loss_sum: f64,
    /// 当前 RSI 值
    rsi: f64,
}

impl RSI {
    /// 创建 RSI 指标，默认 period=14
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "RSI 周期必须大于 0");
        Self {
            period,
            prev_close: None,
            count: 0,
            avg_gain: 0.0,
            avg_loss: 0.0,
            gain_sum: 0.0,
            loss_sum: 0.0,
            rsi: 0.0,
        }
    }

    /// 获取当前 RSI 值
    pub fn value(&self) -> Option<f64> {
        // 需要 period+1 个数据点才能产生第一个 RSI
        if self.count > self.period {
            Some(self.rsi)
        } else {
            None
        }
    }

    /// 根据 avg_gain 和 avg_loss 计算 RSI
    fn calc_rsi(&self) -> f64 {
        if self.avg_loss == 0.0 {
            100.0
        } else {
            let rs = self.avg_gain / self.avg_loss;
            100.0 - 100.0 / (1.0 + rs)
        }
    }
}

impl Indicator for RSI {
    fn name(&self) -> &str {
        "RSI"
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        // 第一个数据点无法计算变化
        let prev = match self.prev_close {
            Some(p) => p,
            None => {
                self.prev_close = Some(value);
                self.count = 1;
                return None;
            }
        };

        self.prev_close = Some(value);
        let change = value - prev;
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };

        self.count += 1;
        let changes_seen = self.count - 1; // 已观测到的变化次数

        if changes_seen < self.period {
            // 初始阶段：累加 gain 和 loss
            self.gain_sum += gain;
            self.loss_sum += loss;
            return None;
        } else if changes_seen == self.period {
            // 第 period 个变化：用简单平均初始化
            self.gain_sum += gain;
            self.loss_sum += loss;
            self.avg_gain = self.gain_sum / self.period as f64;
            self.avg_loss = self.loss_sum / self.period as f64;
        } else {
            // Wilder 平滑：avg = (prev_avg * (period-1) + current) / period
            self.avg_gain = (self.avg_gain * (self.period as f64 - 1.0) + gain) / self.period as f64;
            self.avg_loss = (self.avg_loss * (self.period as f64 - 1.0) + loss) / self.period as f64;
        }

        self.rsi = self.calc_rsi();
        Some(self.rsi)
    }

    fn min_period(&self) -> usize {
        // 需要 period+1 个数据点（1个初始点 + period 个变化）
        self.period + 1
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    fn reset(&mut self) {
        self.prev_close = None;
        self.count = 0;
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.gain_sum = 0.0;
        self.loss_sum = 0.0;
        self.rsi = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_all_up() {
        // 全部上涨 -> RSI 应接近 100
        let mut rsi = RSI::new(3);
        // 需要 4 个数据点（period+1）
        rsi.next(10.0); // count=1, 无变化
        rsi.next(11.0); // change=+1, gain=1
        rsi.next(12.0); // change=+1, gain=1
        let v = rsi.next(13.0).unwrap(); // change=+1, gain=1
        // avg_gain=1.0, avg_loss=0.0 -> RSI=100
        assert!((v - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_rsi_all_down() {
        // 全部下跌 -> RSI 应接近 0
        let mut rsi = RSI::new(3);
        rsi.next(13.0);
        rsi.next(12.0); // change=-1, loss=1
        rsi.next(11.0); // change=-1, loss=1
        let v = rsi.next(10.0).unwrap(); // change=-1, loss=1
        // avg_gain=0, avg_loss=1.0 -> RSI=0
        assert!((v - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_rsi_min_period() {
        let rsi = RSI::new(14);
        assert_eq!(rsi.min_period(), 15);
    }
}
