use super::Indicator;
use crate::core::Bar;

/// ATR (Average True Range)：平均真实波幅
/// True Range = max(high-low, abs(high-prev_close), abs(low-prev_close))
/// ATR = SMA(TR, period)
pub struct ATR {
    period: usize,
    /// 前一根 bar 的收盘价
    prev_close: Option<f64>,
    /// TR 值的滑动窗口
    tr_buffer: Vec<f64>,
    /// 当前 ATR 值
    atr: f64,
}

impl ATR {
    /// 创建 ATR 指标，默认 period=14
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "ATR 周期必须大于 0");
        Self {
            period,
            prev_close: None,
            tr_buffer: Vec::with_capacity(period),
            atr: 0.0,
        }
    }

    /// 获取当前 ATR 值
    pub fn value(&self) -> Option<f64> {
        if self.tr_buffer.len() >= self.period {
            Some(self.atr)
        } else {
            None
        }
    }

    /// 计算 True Range
    fn calc_true_range(&self, high: f64, low: f64) -> f64 {
        match self.prev_close {
            Some(pc) => {
                let hl = high - low;
                let hc = (high - pc).abs();
                let lc = (low - pc).abs();
                hl.max(hc).max(lc)
            }
            None => high - low, // 第一根 bar 用 high - low
        }
    }
}

impl Indicator for ATR {
    fn name(&self) -> &str {
        "ATR"
    }

    fn next(&mut self, _value: f64) -> Option<f64> {
        // ATR 需要 OHLC 数据，单一 close 值不足以计算
        // 使用默认 next_bar 方法，这里返回 None 提示用户使用 next_bar
        None
    }

    fn next_bar(&mut self, bar: &Bar) -> Option<f64> {
        let tr = self.calc_true_range(bar.high, bar.low);
        self.prev_close = Some(bar.close);

        // 使用 Wilder 平滑（类似 RSI）
        if self.tr_buffer.len() < self.period {
            self.tr_buffer.push(tr);
            if self.tr_buffer.len() == self.period {
                // 第一个 ATR = 简单平均
                self.atr = self.tr_buffer.iter().sum::<f64>() / self.period as f64;
                return Some(self.atr);
            }
            return None;
        }

        // Wilder 平滑：ATR = (prev_ATR * (period-1) + TR) / period
        self.atr = (self.atr * (self.period as f64 - 1.0) + tr) / self.period as f64;
        // 保持 buffer 长度为 period（用于 is_ready 判断）
        self.tr_buffer.push(tr);
        if self.tr_buffer.len() > self.period {
            self.tr_buffer.remove(0);
        }
        Some(self.atr)
    }

    fn min_period(&self) -> usize {
        // 需要 period+1 根 bar（第一根用于 prev_close，后续 period 根计算 TR）
        self.period + 1
    }

    fn is_ready(&self) -> bool {
        self.tr_buffer.len() >= self.period
    }

    fn reset(&mut self) {
        self.prev_close = None;
        self.tr_buffer.clear();
        self.atr = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bar(h: f64, l: f64, c: f64) -> Bar {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        Bar::new(dt, (h + l) / 2.0, h, l, c, 1000.0, 0.0)
    }

    #[test]
    fn test_atr_basic() {
        let mut atr = ATR::new(3);
        // min_period = 4
        // 第一根 bar：TR = high - low = 10 - 5 = 5
        let b1 = make_bar(10.0, 5.0, 8.0);
        assert_eq!(atr.next_bar(&b1), None);

        // 第二根 bar：TR = max(12-6, |12-8|, |6-8|) = max(6,4,2) = 6
        let b2 = make_bar(12.0, 6.0, 9.0);
        assert_eq!(atr.next_bar(&b2), None);

        // 第三根 bar：TR = max(11-7, |11-9|, |7-9|) = max(4,2,2) = 4
        let b3 = make_bar(11.0, 7.0, 10.0);
        let v = atr.next_bar(&b3).unwrap();
        // ATR = (5 + 6 + 4) / 3 = 5.0
        assert!((v - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_atr_min_period() {
        let atr = ATR::new(14);
        assert_eq!(atr.min_period(), 15);
    }
}
