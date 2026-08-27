use super::Sizer;

/// 百分比 Sizer：根据可用资金的百分比计算手数
pub struct PercentSizer {
    /// 使用资金的百分比（0.0 ~ 1.0，默认 0.95 即 95%）
    percent: f64,
}

impl PercentSizer {
    /// 创建百分比 Sizer
    /// - percent: 使用资金的比例（0.0 ~ 1.0）
    pub fn new(percent: f64) -> Self {
        assert!(
            percent > 0.0 && percent <= 1.0,
            "percent 必须在 (0.0, 1.0] 范围内"
        );
        Self { percent }
    }
}

impl Default for PercentSizer {
    fn default() -> Self {
        Self { percent: 0.95 }
    }
}

impl Sizer for PercentSizer {
    fn get_size(&self, cash: f64, price: f64, _is_buy: bool) -> i64 {
        if price <= 0.0 || cash <= 0.0 {
            return 0;
        }
        let size = (cash * self.percent / price) as i64;
        // 确保至少返回 1 手（如果资金允许）
        if size == 0 && cash * self.percent >= price {
            1
        } else {
            size
        }
    }
}
