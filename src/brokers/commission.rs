/// 佣金类型枚举：支持多种手续费计算方式
#[derive(Debug, Clone)]
pub enum CommissionType {
    /// 固定百分比：commission = price * size * rate
    Percent { rate: f64 },
    /// 固定金额：commission = fixed_per_trade
    Fixed { amount: f64 },
    /// 固定 + 百分比：commission = fixed + price * size * rate
    FixedPlusPercent { fixed: f64, rate: f64 },
    /// 按手计费：commission = per_share * size（不低于 min）
    PerShare { amount: f64, min: f64 },
}

/// CommissionInfo: 手续费计算
#[derive(Debug, Clone)]
pub struct CommissionInfo {
    /// 佣金类型
    pub commission_type: CommissionType,
}

impl CommissionInfo {
    /// 创建固定百分比佣金（兼容 Phase 1 接口）
    pub fn new(commission_rate: f64) -> Self {
        Self {
            commission_type: CommissionType::Percent { rate: commission_rate },
        }
    }

    /// 从 CommissionType 创建
    pub fn from_type(commission_type: CommissionType) -> Self {
        Self { commission_type }
    }

    /// 计算手续费
    /// size: 交易数量（正数），price: 成交价格
    pub fn calculate(&self, size: i64, price: f64) -> f64 {
        let abs_size = (size as f64).abs();
        match &self.commission_type {
            CommissionType::Percent { rate } => {
                abs_size * price * rate
            }
            CommissionType::Fixed { amount } => {
                *amount
            }
            CommissionType::FixedPlusPercent { fixed, rate } => {
                fixed + abs_size * price * rate
            }
            CommissionType::PerShare { amount, min } => {
                let commission = abs_size * amount;
                if commission < *min { *min } else { commission }
            }
        }
    }
}

impl Default for CommissionInfo {
    fn default() -> Self {
        Self {
            commission_type: CommissionType::Percent { rate: 0.005 }, // 默认 0.5%
        }
    }
}
