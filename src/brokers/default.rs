use std::collections::HashMap;

use crate::core::Bar;
use super::commission::CommissionInfo;
use super::order::{Order, OrderSide, OrderStatus, OrderType};
use super::position::Position;
use super::trade::Trade;

/// 订单通知枚举：Broker 通知 Strategy 订单状态变化
#[derive(Debug, Clone)]
pub enum OrderNotification {
    OrderCompleted(Order),
    OrderCanceled(Order),
    OrderRejected(Order),
}

/// Broker trait：定义 Broker 的统一接口
pub trait Broker {
    /// 提交订单（data_idx 表示操作哪个数据源）
    fn submit_order(&mut self, order: Order, data_idx: usize);

    /// 每根 bar 推送给 Broker，处理挂单撮合
    fn next_bar(&mut self, bar: &Bar, data_idx: usize);

    /// 获取可用现金
    fn get_cash(&self) -> f64;

    /// 获取组合总价值（现金 + 仓位市值）
    fn get_value(&self, bar: &Bar, data_idx: usize) -> f64;

    /// 获取指定数据源的仓位
    fn get_position(&self, data_idx: usize) -> &Position;

    /// 取出并清空通知队列
    fn drain_notifications(&mut self) -> Vec<OrderNotification>;

    /// 获取已完成的交易记录
    fn get_trades(&self) -> &[Trade];

    /// 下一个订单 ID
    fn next_order_id(&mut self) -> u64;
}

/// DefaultBroker：默认 Broker 实现
/// 处理市价单的撮合逻辑
pub struct DefaultBroker {
    cash: f64,
    positions: HashMap<usize, Position>,
    pending_orders: Vec<(Order, usize)>,   // (order, data_idx)
    completed_trades: Vec<Trade>,
    commission_info: CommissionInfo,
    notifications: Vec<OrderNotification>,
    next_id: u64,
    trade_id: u64,
    // 记录每个 data_idx 的当前 bar（用于撮合时获取价格）
    current_bars: HashMap<usize, Bar>,
}

impl DefaultBroker {
    pub fn new(cash: f64, commission_info: CommissionInfo) -> Self {
        Self {
            cash,
            positions: HashMap::new(),
            pending_orders: Vec::new(),
            completed_trades: Vec::new(),
            commission_info,
            notifications: Vec::new(),
            next_id: 1,
            trade_id: 1,
            current_bars: HashMap::new(),
        }
    }

    /// 尝试执行市价单
    fn try_execute_market_order(&mut self, order: &mut Order, bar: &Bar, data_idx: usize) {
        let exec_price = bar.open; // 市价单在 open 价执行
        let commission = self.commission_info.calculate(order.size, exec_price);

        match order.side {
            OrderSide::Buy => {
                let cost = exec_price * order.size as f64 + commission;
                if self.cash >= cost {
                    // 资金充足，执行买入
                    self.cash -= cost;
                    let pos = self.positions.entry(data_idx).or_insert_with(Position::new);
                    let prev_size = pos.size;
                    let prev_price = pos.price;
                    pos.update(order.size, exec_price);

                    // 记录交易
                    if prev_size == 0 {
                        // 新开仓
                        let trade = Trade::new(self.trade_id, bar.datetime, exec_price, order.size);
                        self.trade_id += 1;
                        self.completed_trades.push(trade);
                    }

                    order.execute(bar.datetime, exec_price, order.size, commission);
                    self.notifications.push(OrderNotification::OrderCompleted(order.clone()));
                } else {
                    // 资金不足，拒绝
                    order.reject();
                    self.notifications.push(OrderNotification::OrderRejected(order.clone()));
                }
            }
            OrderSide::Sell => {
                let pos = self.positions.entry(data_idx).or_insert_with(Position::new);
                if pos.size >= order.size {
                    // 仓位充足，执行卖出
                    let revenue = exec_price * order.size as f64 - commission;
                    self.cash += revenue;

                    // 如果有关联的开仓交易，平仓它
                    let trade = self.completed_trades.iter_mut()
                        .find(|t| !t.is_closed());
                    if let Some(trade) = trade {
                        trade.close(bar.datetime, exec_price, commission);
                    }

                    pos.update(-(order.size), exec_price);
                    order.execute(bar.datetime, exec_price, order.size, commission);
                    self.notifications.push(OrderNotification::OrderCompleted(order.clone()));
                } else {
                    // 仓位不足，拒绝
                    order.reject();
                    self.notifications.push(OrderNotification::OrderRejected(order.clone()));
                }
            }
        }
    }
}

impl Broker for DefaultBroker {
    fn submit_order(&mut self, mut order: Order, data_idx: usize) {
        // 获取当前 bar 时间用于记录提交时间
        if let Some(bar) = self.current_bars.get(&data_idx) {
            order.submit(bar.datetime);
        }
        order.accept();
        self.pending_orders.push((order, data_idx));
    }

    fn next_bar(&mut self, bar: &Bar, data_idx: usize) {
        // 更新当前 bar
        self.current_bars.insert(data_idx, bar.clone());

        // 更新持仓的当前价格
        if let Some(pos) = self.positions.get_mut(&data_idx) {
            pos.current_price = bar.close;
        }

        // 处理所有挂单
        let mut remaining = Vec::new();
        for (mut order, didx) in self.pending_orders.drain(..) {
            if !order.is_active() {
                continue;
            }
            match order.order_type {
                OrderType::Market => {
                    if didx == data_idx {
                        self.try_execute_market_order(&mut order, bar, didx);
                    } else {
                        remaining.push((order, didx));
                    }
                }
                _ => {
                    // Phase 1 仅支持市价单，其他类型暂存
                    remaining.push((order, didx));
                }
            }
        }
        self.pending_orders = remaining;
    }

    fn get_cash(&self) -> f64 {
        self.cash
    }

    fn get_value(&self, bar: &Bar, data_idx: usize) -> f64 {
        let position_value = self.positions
            .get(&data_idx)
            .map(|pos| pos.size as f64 * bar.close)
            .unwrap_or(0.0);
        self.cash + position_value
    }

    fn get_position(&self, data_idx: usize) -> &Position {
        // 如果不存在，返回一个静态空仓位
        // 注意：这里用了一个小技巧，如果不存在就临时创建并 leak（但更好的方式是用 get_or_default）
        self.positions.get(&data_idx).unwrap_or_else(|| {
            // 返回一个静态默认值的引用 —— 由于不能返回临时引用，改用下面的方法
            // 实际使用中应该先 insert 再 get
            &EMPTY_POSITION
        })
    }

    fn drain_notifications(&mut self) -> Vec<OrderNotification> {
        std::mem::take(&mut self.notifications)
    }

    fn get_trades(&self) -> &[Trade] {
        &self.completed_trades
    }

    fn next_order_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// 全局静态空仓位（用于 get_position 返回引用）
static EMPTY_POSITION: Position = Position {
    size: 0,
    price: 0.0,
    current_price: 0.0,
};
