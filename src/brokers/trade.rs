use chrono::NaiveDateTime;

/// Trade 结构：记录一次完整的开仓-平仓交易
#[derive(Debug, Clone)]
pub struct Trade {
    pub id: u64,
    pub entry_dt: NaiveDateTime,
    pub exit_dt: Option<NaiveDateTime>,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: i64,
    pub pnl: f64,
    pub commission: f64,
}

impl Trade {
    /// 创建新交易记录（开仓时）
    pub fn new(id: u64, entry_dt: NaiveDateTime, entry_price: f64, size: i64) -> Self {
        Self {
            id,
            entry_dt,
            exit_dt: None,
            entry_price,
            exit_price: 0.0,
            size,
            pnl: 0.0,
            commission: 0.0,
        }
    }

    /// 平仓并计算盈亏
    pub fn close(&mut self, exit_dt: NaiveDateTime, exit_price: f64, commission: f64) {
        self.exit_dt = Some(exit_dt);
        self.exit_price = exit_price;
        self.commission = commission;
        // 盈亏 = (卖出价 - 买入价) * 数量 - 手续费
        self.pnl = (exit_price - self.entry_price) * self.size as f64 - commission;
    }

    /// 交易是否已平仓
    pub fn is_closed(&self) -> bool {
        self.exit_dt.is_some()
    }
}
