use super::Sizer;
use std::cell::Cell;

/// 基于 ATR 的风险定价 Sizer
///
/// 根据风险百分比和 ATR 值计算手数：
/// - 风险金额 = cash * risk_percent
/// - 每手风险 = atr_value（假设止损设在 1 个 ATR 距离）
/// - size = (risk_amount / atr_value) as i64
pub struct ATRSizer {
    /// 风险百分比（默认 0.01 即 1%）
    risk_percent: f64,
    /// 当前 ATR 值（可在运行时更新）
    atr_value: Cell<f64>,
}

impl ATRSizer {
    /// 创建 ATR Sizer
    /// - risk_percent: 每笔交易风险占总资金的比例
    /// - atr_value: 初始 ATR 值
    pub fn new(risk_percent: f64, atr_value: f64) -> Self {
        Self {
            risk_percent,
            atr_value: Cell::new(atr_value),
        }
    }

    /// 更新 ATR 值（在策略运行中动态调用）
    pub fn set_atr_value(&self, value: f64) {
        self.atr_value.set(value);
    }

    /// 获取当前 ATR 值
    pub fn get_atr_value(&self) -> f64 {
        self.atr_value.get()
    }
}

impl Default for ATRSizer {
    fn default() -> Self {
        Self {
            risk_percent: 0.01,
            atr_value: Cell::new(1.0),
        }
    }
}

impl Sizer for ATRSizer {
    fn get_size(&self, cash: f64, _price: f64, _is_buy: bool) -> i64 {
        let atr = self.atr_value.get();
        if atr <= 0.0 || cash <= 0.0 {
            return 0;
        }
        let risk_amount = cash * self.risk_percent;
        let size = (risk_amount / atr) as i64;
        // 确保至少返回 1 手（如果风险金额允许）
        if size == 0 && risk_amount >= atr {
            1
        } else {
            size
        }
    }
}
