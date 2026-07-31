use chrono::NaiveDateTime;

/// 订单类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Market,   // 市价单
    Limit,    // 限价单
    Stop,     // 止损单
    StopLimit,// 止损限价单
    Close,    // 收盘单
}

/// 订单方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// 订单状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Created,   // 刚创建
    Submitted, // 已提交
    Accepted,  // 已接受
    Completed, // 已完成
    Canceled,  // 已取消
    Rejected,  // 被拒绝
    Expired,   // 已过期
}

/// Order 结构：表示一个交易订单
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub size: i64,
    pub price: Option<f64>,         // 限价单价格
    pub limit_price: Option<f64>,   // 止损限价单限价
    pub stop_price: Option<f64>,    // 止损价
    pub status: OrderStatus,
    pub created_dt: Option<NaiveDateTime>,
    pub executed_dt: Option<NaiveDateTime>,
    pub executed_price: f64,
    pub executed_size: i64,
    pub commission: f64,
}

impl Order {
    /// 创建市价单
    pub fn new_market(id: u64, side: OrderSide, size: i64) -> Self {
        Self {
            id,
            order_type: OrderType::Market,
            side,
            size,
            price: None,
            limit_price: None,
            stop_price: None,
            status: OrderStatus::Created,
            created_dt: None,
            executed_dt: None,
            executed_price: 0.0,
            executed_size: 0,
            commission: 0.0,
        }
    }

    /// 创建限价单
    pub fn new_limit(id: u64, side: OrderSide, size: i64, price: f64) -> Self {
        Self {
            id,
            order_type: OrderType::Limit,
            side,
            size,
            price: Some(price),
            limit_price: None,
            stop_price: None,
            status: OrderStatus::Created,
            created_dt: None,
            executed_dt: None,
            executed_price: 0.0,
            executed_size: 0,
            commission: 0.0,
        }
    }

    /// 提交订单
    pub fn submit(&mut self, dt: NaiveDateTime) {
        self.status = OrderStatus::Submitted;
        self.created_dt = Some(dt);
    }

    /// 接受订单
    pub fn accept(&mut self) {
        self.status = OrderStatus::Accepted;
    }

    /// 执行订单（成交）
    pub fn execute(&mut self, dt: NaiveDateTime, price: f64, size: i64, commission: f64) {
        self.status = OrderStatus::Completed;
        self.executed_dt = Some(dt);
        self.executed_price = price;
        self.executed_size = size;
        self.commission = commission;
    }

    /// 取消订单
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Canceled;
    }

    /// 拒绝订单
    pub fn reject(&mut self) {
        self.status = OrderStatus::Rejected;
    }

    /// 订单是否已完成
    pub fn is_completed(&self) -> bool {
        self.status == OrderStatus::Completed
    }

    /// 订单是否仍然活跃（未终态）
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Created | OrderStatus::Submitted | OrderStatus::Accepted
        )
    }
}
