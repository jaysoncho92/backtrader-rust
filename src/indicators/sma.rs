use std::collections::VecDeque;

use super::Indicator;

/// SMA (Simple Moving Average)：简单移动平均线
/// 维护一个固定窗口的滑动平均值
pub struct SMA {
    period: usize,
    buffer: VecDeque<f64>,
    sum: f64,
}

impl SMA {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "SMA 周期必须大于 0");
        Self {
            period,
            buffer: VecDeque::with_capacity(period),
            sum: 0.0,
        }
    }

    /// 获取当前 SMA 值（未就绪时返回 None）
    pub fn value(&self) -> Option<f64> {
        if self.buffer.len() >= self.period {
            Some(self.sum / self.period as f64)
        } else {
            None
        }
    }
}

impl Indicator for SMA {
    fn name(&self) -> &str {
        "SMA"
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        // 如果缓冲区已满，先减去最旧的值
        if self.buffer.len() >= self.period {
            let old = self.buffer.pop_front().unwrap();
            self.sum -= old;
        }
        self.buffer.push_back(value);
        self.sum += value;

        if self.buffer.len() >= self.period {
            Some(self.sum / self.period as f64)
        } else {
            None
        }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let mut sma = SMA::new(3);
        assert_eq!(sma.next(1.0), None);
        assert_eq!(sma.next(2.0), None);
        // 第三个值到达，SMA = (1+2+3)/3 = 2.0
        let v = sma.next(3.0).unwrap();
        assert!((v - 2.0).abs() < 1e-9);
        // 第四个值，SMA = (2+3+4)/3 = 3.0
        let v = sma.next(4.0).unwrap();
        assert!((v - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_sma_reset() {
        let mut sma = SMA::new(2);
        sma.next(10.0);
        sma.next(20.0);
        assert!(sma.is_ready());
        sma.reset();
        assert!(!sma.is_ready());
    }
}
