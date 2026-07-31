/// CommissionInfo: 手续费计算
#[derive(Debug, Clone)]
pub struct CommissionInfo {
    /// 手续费率（如 0.005 表示 0.5%）
    pub commission_rate: f64,
}

impl CommissionInfo {
    pub fn new(commission_rate: f64) -> Self {
        Self { commission_rate }
    }

    /// 计算手续费 = size * price * rate
    pub fn calculate(&self, size: i64, price: f64) -> f64 {
        (size as f64).abs() * price * self.commission_rate
    }
}

impl Default for CommissionInfo {
    fn default() -> Self {
        Self {
            commission_rate: 0.005, // 默认 0.5%
        }
    }
}
