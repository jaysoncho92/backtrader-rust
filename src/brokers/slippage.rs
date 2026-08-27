/// 滑点模型：模拟实际交易中的价格滑移
///
/// 市价单执行时，实际成交价可能偏离理论价格。
/// 滑点只应用于市价单；Limit/Stop 单已经是确定价格，不应用滑点。

/// 滑点枚举
#[derive(Debug, Clone)]
pub enum Slippage {
    /// 固定滑点（价格点数）
    /// 买入时价格向上偏移，卖出时价格向下偏移
    Fixed(f64),
    /// 百分比滑点
    /// 买入时价格向上偏移百分比，卖出时价格向下偏移百分比
    Percent(f64),
}

impl Slippage {
    /// 计算实际执行价格
    /// is_buy: true 时价格向上滑（对买方不利），false 时向下滑（对卖方不利）
    pub fn apply(&self, price: f64, is_buy: bool) -> f64 {
        match self {
            Slippage::Fixed(amount) => {
                if is_buy {
                    price + amount
                } else {
                    price - amount
                }
            }
            Slippage::Percent(pct) => {
                if is_buy {
                    price * (1.0 + pct)
                } else {
                    price * (1.0 - pct)
                }
            }
        }
    }
}
