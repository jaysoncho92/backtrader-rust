use super::Sizer;

/// 固定手数 Sizer：始终返回预设的固定手数
pub struct FixedSizer {
    stake: i64,
}

impl FixedSizer {
    /// 创建固定手数 Sizer
    /// - stake: 每次下单的固定手数（默认 100）
    pub fn new(stake: i64) -> Self {
        Self { stake }
    }

    /// 使用默认手数 100
    pub fn default_stake() -> Self {
        Self { stake: 100 }
    }
}

impl Default for FixedSizer {
    fn default() -> Self {
        Self::default_stake()
    }
}

impl Sizer for FixedSizer {
    fn get_size(&self, _cash: f64, _price: f64, _is_buy: bool) -> i64 {
        self.stake
    }
}
