/// Position 结构：表示当前持仓状态
#[derive(Debug, Clone)]
pub struct Position {
    /// 持仓数量（正数=多头，负数=空头）
    pub size: i64,
    /// 持仓均价
    pub price: f64,
    /// 当前市场价（用于计算浮动盈亏）
    pub current_price: f64,
}

impl Position {
    /// 创建空仓位
    pub fn new() -> Self {
        Self {
            size: 0,
            price: 0.0,
            current_price: 0.0,
        }
    }

    /// 更新仓位（处理加仓/减仓）
    /// size: 本次交易数量（正数=买入，负数=卖出）
    /// price: 本次成交价格
    pub fn update(&mut self, size: i64, price: f64) {
        let new_size = self.size + size;

        if new_size == 0 {
            // 完全平仓
            self.size = 0;
            self.price = 0.0;
        } else if (self.size > 0 && size < 0) || (self.size < 0 && size > 0) {
            // 减仓或反手
            if new_size.signum() == self.size.signum() {
                // 部分减仓，均价不变
                self.size = new_size;
            } else {
                // 反手，以当前成交价作为新均价
                self.size = new_size;
                self.price = price;
            }
        } else {
            // 加仓：计算加权平均价
            if self.size == 0 {
                self.price = price;
                self.size = size;
            } else {
                let total_cost = self.price * self.size as f64 + price * size as f64;
                self.size = new_size;
                self.price = total_cost / self.size as f64;
            }
        }
        self.current_price = price;
    }

    /// 以指定价格平仓
    pub fn close(&mut self, price: f64) {
        self.current_price = price;
        self.size = 0;
        self.price = 0.0;
    }

    /// 是否有持仓
    pub fn is_open(&self) -> bool {
        self.size != 0
    }

    /// 当前市值
    pub fn market_value(&self) -> f64 {
        self.size.abs() as f64 * self.current_price
    }

    /// 浮动盈亏（未实现盈亏）
    pub fn unrealized_pnl(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        (self.current_price - self.price) * self.size as f64
    }

    /// 以指定平仓价格计算总盈亏
    pub fn pnl(&self, close_price: f64) -> f64 {
        (close_price - self.price) * self.size as f64
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_buy() {
        let mut pos = Position::new();
        pos.update(100, 50.0);
        assert_eq!(pos.size, 100);
        assert!((pos.price - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_position_add() {
        let mut pos = Position::new();
        pos.update(100, 50.0);
        pos.update(100, 60.0);
        assert_eq!(pos.size, 200);
        assert!((pos.price - 55.0).abs() < 1e-9);
    }

    #[test]
    fn test_position_close() {
        let mut pos = Position::new();
        pos.update(100, 50.0);
        pos.update(-100, 60.0);
        assert_eq!(pos.size, 0);
    }

    #[test]
    fn test_position_pnl() {
        let mut pos = Position::new();
        pos.update(100, 50.0);
        pos.current_price = 60.0;
        assert!((pos.unrealized_pnl() - 1000.0).abs() < 1e-9);
        assert!((pos.pnl(60.0) - 1000.0).abs() < 1e-9);
    }
}
