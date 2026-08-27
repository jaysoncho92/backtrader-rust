use std::collections::{HashMap, HashSet};

use crate::core::Bar;
use super::commission::CommissionInfo;
use super::order::{Order, OrderSide, OrderStatus, OrderType};
use super::position::Position;
use super::slippage::Slippage;
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

/// 订单执行结果
enum ExecResult {
    /// 订单已成交
    Filled,
    /// 订单被拒绝（资金/仓位不足）
    Rejected,
}

/// DefaultBroker：默认 Broker 实现
/// 支持市价单、限价单、止损单、止损限价单、OCO、Bracket 订单
pub struct DefaultBroker {
    cash: f64,
    positions: HashMap<usize, Position>,
    pending_orders: Vec<(Order, usize)>,   // (order, data_idx)
    completed_trades: Vec<Trade>,
    commission_info: CommissionInfo,
    notifications: Vec<OrderNotification>,
    next_id: u64,
    trade_id: u64,
    /// 记录每个 data_idx 的当前 bar
    current_bars: HashMap<usize, Bar>,
    /// 滑点配置（仅对市价单生效）
    slippage: Option<Slippage>,
    /// 已激活的 Bracket 父订单 ID 集合
    /// 子订单（parent_id 在集合中）才能被处理
    activated_parents: HashSet<u64>,
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
            slippage: None,
            activated_parents: HashSet::new(),
        }
    }

    /// 设置滑点模型
    pub fn set_slippage(&mut self, slippage: Slippage) {
        self.slippage = Some(slippage);
    }

    /// 创建 Bracket 订单：主单 + 止盈单 + 止损单
    /// 返回 (entry_id, take_profit_id, stop_loss_id)
    pub fn bracket_order(
        &mut self,
        side: OrderSide,
        size: i64,
        entry_price: f64,
        take_profit_price: f64,
        stop_loss_price: f64,
        data_idx: usize,
    ) -> (u64, u64, u64) {
        let entry_id = self.next_order_id();
        let tp_id = self.next_order_id();
        let sl_id = self.next_order_id();

        // 主单：限价入场单
        let entry = Order::new_limit(entry_id, side, size, entry_price);

        // 止盈单：限价单（方向与主单相反）
        let exit_side = match side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        };
        let mut tp = Order::new_limit(tp_id, exit_side, size, take_profit_price);
        tp.parent_id = Some(entry_id);
        // 修复: 为止盈/止损子单设置相同的 OCO 组，确保一张成交后另一张被自动取消
        tp.oco_group = Some(entry_id);

        // 止损单：止损单（方向与主单相反）
        let mut sl = Order::new_stop(sl_id, exit_side, size, stop_loss_price);
        sl.parent_id = Some(entry_id);
        // 修复: 与 tp 同一 OCO 组，避免 tp 成交后 sl 被错误 "Rejected"
        sl.oco_group = Some(entry_id);

        // 提交所有订单
        self.submit_order_internal(entry, data_idx);
        self.submit_order_internal(tp, data_idx);
        self.submit_order_internal(sl, data_idx);

        (entry_id, tp_id, sl_id)
    }

    /// 内部提交订单（不经过 trait，用于 bracket_order 等内部调用）
    fn submit_order_internal(&mut self, mut order: Order, data_idx: usize) {
        if let Some(bar) = self.current_bars.get(&data_idx) {
            order.submit(bar.datetime);
        }
        order.accept();
        self.pending_orders.push((order, data_idx));
    }

    /// 以指定价格尝试执行订单（核心撮合逻辑）
    /// 返回执行结果
    fn try_execute_at_price(
        &mut self,
        order: &mut Order,
        bar: &Bar,
        data_idx: usize,
        exec_price: f64,
    ) -> ExecResult {
        // 修复 #6: size=0 的订单不应被执行，直接拒绝
        if order.size <= 0 {
            order.reject();
            self.notifications
                .push(OrderNotification::OrderRejected(order.clone()));
            return ExecResult::Rejected;
        }

        let commission = self.commission_info.calculate(order.size, exec_price);

        match order.side {
            OrderSide::Buy => {
                let cost = exec_price * order.size as f64 + commission;
                if self.cash < cost {
                    // 资金不足，拒绝
                    order.reject();
                    self.notifications
                        .push(OrderNotification::OrderRejected(order.clone()));
                    return ExecResult::Rejected;
                }

                // 资金充足，执行买入
                self.cash -= cost;
                let pos = self
                    .positions
                    .entry(data_idx)
                    .or_insert_with(Position::new);
                let prev_size = pos.size;
                pos.update(order.size, exec_price);

                // 新开仓时记录交易
                if prev_size == 0 {
                    let trade =
                        Trade::new(self.trade_id, bar.datetime, exec_price, order.size);
                    self.trade_id += 1;
                    self.completed_trades.push(trade);
                }

                order.execute(bar.datetime, exec_price, order.size, commission);
                self.notifications
                    .push(OrderNotification::OrderCompleted(order.clone()));

                // 激活 Bracket 子订单
                if order.is_completed() {
                    self.activated_parents.insert(order.id);
                }
                ExecResult::Filled
            }
            OrderSide::Sell => {
                let pos = self
                    .positions
                    .entry(data_idx)
                    .or_insert_with(Position::new);
                if pos.size < order.size {
                    // 仓位不足，拒绝
                    order.reject();
                    self.notifications
                        .push(OrderNotification::OrderRejected(order.clone()));
                    return ExecResult::Rejected;
                }

                // 仓位充足，执行卖出
                let revenue = exec_price * order.size as f64 - commission;
                self.cash += revenue;

                // 平仓关联的交易记录
                let trade = self
                    .completed_trades
                    .iter_mut()
                    .find(|t| !t.is_closed());
                if let Some(trade) = trade {
                    trade.close(bar.datetime, exec_price, commission);
                }

                pos.update(-(order.size), exec_price);
                order.execute(bar.datetime, exec_price, order.size, commission);
                self.notifications
                    .push(OrderNotification::OrderCompleted(order.clone()));

                // 激活 Bracket 子订单
                if order.is_completed() {
                    self.activated_parents.insert(order.id);
                }
                ExecResult::Filled
            }
        }
    }

    /// 处理 Stop 单：检查是否触发并执行
    fn try_process_stop(
        &mut self,
        order: &mut Order,
        bar: &Bar,
        data_idx: usize,
        _bar_idx: usize,
    ) -> bool {
        let stop_price = order.stop_price.unwrap();

        let triggered = match order.side {
            OrderSide::Buy => bar.high >= stop_price,
            OrderSide::Sell => bar.low <= stop_price,
        };

        if !triggered {
            return true; // 未触发，继续挂单
        }

        // 触发后以市价执行（open 价，含滑点）
        let base_price = bar.open;
        let exec_price = if let Some(ref slippage) = self.slippage {
            slippage.apply(base_price, order.side == OrderSide::Buy)
        } else {
            base_price
        };
        let result = self.try_execute_at_price(order, bar, data_idx, exec_price);
        !matches!(result, ExecResult::Filled | ExecResult::Rejected)
    }

    /// 处理 Limit 单：检查是否在 bar 范围内可执行
    fn try_process_limit(
        &mut self,
        order: &mut Order,
        bar: &Bar,
        data_idx: usize,
    ) -> bool {
        let limit_price = order.price.unwrap();

        let can_execute = match order.side {
            // Buy Limit：当 bar.low ≤ limit_price 时以 limit_price 执行
            OrderSide::Buy => bar.low <= limit_price,
            // Sell Limit：当 bar.high ≥ limit_price 时以 limit_price 执行
            OrderSide::Sell => bar.high >= limit_price,
        };

        if !can_execute {
            return true; // 条件不满足，继续挂单
        }

        let result = self.try_execute_at_price(order, bar, data_idx, limit_price);
        !matches!(result, ExecResult::Filled | ExecResult::Rejected)
    }

    /// 处理 StopLimit 单：先检查 Stop 触发，再按 Limit 规则执行
    fn try_process_stop_limit(
        &mut self,
        order: &mut Order,
        bar: &Bar,
        data_idx: usize,
    ) -> bool {
        let stop_price = order.stop_price.unwrap();
        let limit_price = order.price.unwrap();

        // 如果尚未触发，检查是否需要触发
        if !order.triggered {
            let should_trigger = match order.side {
                OrderSide::Buy => bar.high >= stop_price,
                OrderSide::Sell => bar.low <= stop_price,
            };
            if should_trigger {
                order.triggered = true;
                // 触发后标记为 Limit 单逻辑（下面继续处理）
            } else {
                return true; // 未触发，继续挂单
            }
        }

        // 触发后按 Limit 单规则执行
        let can_execute = match order.side {
            OrderSide::Buy => bar.low <= limit_price,
            OrderSide::Sell => bar.high >= limit_price,
        };

        if !can_execute {
            return true; // 触发但限价未满足，继续挂单
        }

        let result = self.try_execute_at_price(order, bar, data_idx, limit_price);
        !matches!(result, ExecResult::Filled | ExecResult::Rejected)
    }
}

impl Broker for DefaultBroker {
    fn submit_order(&mut self, mut order: Order, data_idx: usize) {
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

        // 取出所有挂单
        let pending = std::mem::take(&mut self.pending_orders);
        let mut remaining = Vec::new();
        let mut filled_oco_groups: HashSet<u64> = HashSet::new();
        let mut cancelled_parent_ids: HashSet<u64> = HashSet::new();

        // 按优先级处理订单：
        // 1. Stop 单触发检查
        // 2. Limit 单处理
        // 3. 市价单处理
        // 这里简化为按顺序处理（同一 bar 内所有订单按挂单顺序处理）
        for (mut order, didx) in pending {
            // 跳过非活跃订单
            if !order.is_active() {
                continue;
            }

            // OCO 组检查：如果该订单所在组已有订单成交，则取消
            if let Some(group_id) = order.oco_group {
                if filled_oco_groups.contains(&group_id) {
                    order.cancel();
                    self.notifications
                        .push(OrderNotification::OrderCanceled(order.clone()));
                    continue;
                }
            }

            // Bracket 子订单检查：父订单取消则取消子订单
            if let Some(pid) = order.parent_id {
                if cancelled_parent_ids.contains(&pid) {
                    order.cancel();
                    self.notifications
                        .push(OrderNotification::OrderCanceled(order.clone()));
                    continue;
                }
                // 子订单等待父订单激活
                if !self.activated_parents.contains(&pid) {
                    remaining.push((order, didx));
                    continue;
                }
            }

            // 只处理当前 data_idx 的订单
            if didx != data_idx {
                remaining.push((order, didx));
                continue;
            }

            let keep = match order.order_type {
                OrderType::Stop => {
                    self.try_process_stop(&mut order, bar, didx, 0)
                }
                OrderType::Limit => {
                    self.try_process_limit(&mut order, bar, didx)
                }
                OrderType::StopLimit => {
                    self.try_process_stop_limit(&mut order, bar, didx)
                }
                OrderType::Market => {
                    let base_price = bar.open;
                    let exec_price = if let Some(ref slippage) = self.slippage {
                        slippage.apply(base_price, order.side == OrderSide::Buy)
                    } else {
                        base_price
                    };
                    let result =
                        self.try_execute_at_price(&mut order, bar, didx, exec_price);
                    !matches!(result, ExecResult::Filled | ExecResult::Rejected)
                }
                OrderType::Close => true,
            };

            // 如果订单成交且有 OCO 组，记录该组已被填充
            if order.is_completed() {
                if let Some(group_id) = order.oco_group {
                    filled_oco_groups.insert(group_id);
                }
            }

            // 如果订单被取消且有子订单（Bracket），标记父订单已取消
            if order.status == OrderStatus::Canceled {
                cancelled_parent_ids.insert(order.id);
            }

            if keep {
                remaining.push((order, didx));
            }
        }

        // 再次扫描：取消 OCO 组内剩余的活跃订单
        let mut final_remaining = Vec::new();
        for (mut order, didx) in remaining {
            if order.is_active() {
                if let Some(group_id) = order.oco_group {
                    if filled_oco_groups.contains(&group_id) {
                        order.cancel();
                        self.notifications
                            .push(OrderNotification::OrderCanceled(order.clone()));
                        continue;
                    }
                }
                // 取消依赖已取消父订单的子订单
                if let Some(pid) = order.parent_id {
                    if cancelled_parent_ids.contains(&pid) {
                        order.cancel();
                        self.notifications
                            .push(OrderNotification::OrderCanceled(order.clone()));
                        continue;
                    }
                }
            }
            final_remaining.push((order, didx));
        }

        self.pending_orders = final_remaining;
    }

    fn get_cash(&self) -> f64 {
        self.cash
    }

    fn get_value(&self, bar: &Bar, data_idx: usize) -> f64 {
        let position_value = self
            .positions
            .get(&data_idx)
            .map(|pos| pos.size as f64 * bar.close)
            .unwrap_or(0.0);
        self.cash + position_value
    }

    fn get_position(&self, data_idx: usize) -> &Position {
        self.positions.get(&data_idx).unwrap_or(&EMPTY_POSITION)
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
