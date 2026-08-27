use crate::core::{Bar, TimeSeries};
use crate::brokers::{Broker, Order, OrderSide, Position};
use crate::sizers::Sizer;

/// Strategy trait：定义策略的生命周期回调
pub trait Strategy {
    /// 初始化阶段：创建指标、预计算数据
    fn init(&mut self, ctx: &mut Context);

    /// 每根 bar 调用（指标全部就绪后）
    fn next(&mut self, ctx: &mut Context);

    /// 指标尚未就绪时的回调（可选，默认空）
    fn prenext(&mut self, _ctx: &mut Context) {}

    /// 回测结束时的清理回调（可选，默认空）
    fn stop(&mut self, _ctx: &mut Context) {}
}

// 确保 Strategy 可以 Send（用于并行优化）
// Strategy 本身不需要 Send，但在 optimizer 中我们要求 S: Strategy + Send

/// Context：Strategy 访问数据和 Broker 的桥梁
/// 由 Cerebro 引擎在每根 bar 时构建并传给 Strategy
/// 修复: data 改为引用 &'a [TimeSeries<Bar>]，避免每根 bar 克隆所有数据导致 O(N²) 性能问题
pub struct Context<'a> {
    /// 多数据源的时序数据（引用，不再克隆）
    pub data: &'a [TimeSeries<Bar>],
    /// Broker 引用（用于下单、查询资金）
    pub broker: &'a mut dyn Broker,
    /// 当前处理到的 bar 索引（从 0 开始）
    pub current_bar: usize,
    /// Sizer 引用（可选，用于计算默认下单手数）
    pub sizer: Option<&'a dyn Sizer>,
}

impl<'a> Context<'a> {
    pub fn new(
        data: &'a [TimeSeries<Bar>],
        broker: &'a mut dyn Broker,
        current_bar: usize,
    ) -> Self {
        Self {
            data,
            broker,
            current_bar,
            sizer: None,
        }
    }

    /// 创建带 Sizer 的 Context
    pub fn with_sizer(
        data: &'a [TimeSeries<Bar>],
        broker: &'a mut dyn Broker,
        current_bar: usize,
        sizer: Option<&'a dyn Sizer>,
    ) -> Self {
        Self {
            data,
            broker,
            current_bar,
            sizer,
        }
    }

    /// 获取指定数据源的时序数据
    pub fn data(&self, idx: usize) -> &TimeSeries<Bar> {
        &self.data[idx]
    }

    /// 获取指定数据源的持仓
    pub fn position(&self, data_idx: usize) -> &Position {
        self.broker.get_position(data_idx)
    }

    /// 提交买入市价单
    pub fn buy(&mut self, data_idx: usize, size: i64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_market(id, OrderSide::Buy, size);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交卖出市价单
    pub fn sell(&mut self, data_idx: usize, size: i64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_market(id, OrderSide::Sell, size);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 使用 Sizer 计算默认手数的买入
    /// 若未设置 Sizer，使用 95% 资金计算
    pub fn buy_default(&mut self, data_idx: usize) -> u64 {
        let size = self.calc_default_size(data_idx, true);
        self.buy(data_idx, size)
    }

    /// 使用 Sizer 计算默认手数的卖出
    /// 若未设置 Sizer，使用 95% 资金计算
    pub fn sell_default(&mut self, data_idx: usize) -> u64 {
        let size = self.calc_default_size(data_idx, false);
        self.sell(data_idx, size)
    }

    /// 计算默认手数
    fn calc_default_size(&self, data_idx: usize, is_buy: bool) -> i64 {
        let price = self.data[data_idx]
            .last()
            .map(|b| b.close)
            .unwrap_or(0.0);
        let cash = self.broker.get_cash();

        if let Some(sizer) = self.sizer {
            sizer.get_size(cash, price, is_buy)
        } else {
            // 默认使用 95% 资金
            if price > 0.0 {
                (cash * 0.95 / price) as i64
            } else {
                0
            }
        }
    }

    /// 平仓（卖出全部持仓）
    pub fn close(&mut self, data_idx: usize) {
        let pos = self.broker.get_position(data_idx);
        let size = pos.size;
        if size > 0 {
            self.sell(data_idx, size);
        }
    }

    /// 获取可用现金
    pub fn cash(&self) -> f64 {
        self.broker.get_cash()
    }

    /// 获取组合总价值
    pub fn portfolio_value(&self, data_idx: usize) -> f64 {
        if let Some(bar) = self.data[data_idx].last() {
            self.broker.get_value(bar, data_idx)
        } else {
            self.broker.get_cash()
        }
    }

    // ========== 高级订单便捷 API（修复 #4）==========

    /// 提交限价买入单
    pub fn buy_limit(&mut self, data_idx: usize, size: i64, price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_limit(id, OrderSide::Buy, size, price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交限价卖出单
    pub fn sell_limit(&mut self, data_idx: usize, size: i64, price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_limit(id, OrderSide::Sell, size, price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交止损买入单（Stop 单，触发后以市价执行）
    pub fn buy_stop(&mut self, data_idx: usize, size: i64, stop_price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_stop(id, OrderSide::Buy, size, stop_price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交止损卖出单（Stop 单，触发后以市价执行）
    pub fn sell_stop(&mut self, data_idx: usize, size: i64, stop_price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_stop(id, OrderSide::Sell, size, stop_price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交止损限价买入单（StopLimit 单）
    pub fn buy_stop_limit(&mut self, data_idx: usize, size: i64, stop_price: f64, limit_price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_stop_limit(id, OrderSide::Buy, size, stop_price, limit_price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交止损限价卖出单（StopLimit 单）
    pub fn sell_stop_limit(&mut self, data_idx: usize, size: i64, stop_price: f64, limit_price: f64) -> u64 {
        let id = self.broker.next_order_id();
        let order = Order::new_stop_limit(id, OrderSide::Sell, size, stop_price, limit_price);
        self.broker.submit_order(order, data_idx);
        id
    }

    /// 提交 Bracket 买入单：限价入场 + 止盈 + 止损
    /// 返回 (entry_id, take_profit_id, stop_loss_id)
    pub fn bracket_buy(&mut self, data_idx: usize, size: i64, entry_price: f64, take_profit: f64, stop_loss: f64) -> (u64, u64, u64) {
        // 通过 trait object 调用 DefaultBroker 的 bracket_order
        // 由于 Broker trait 不包含 bracket_order，这里手动构造三张订单
        let entry_id = self.broker.next_order_id();
        let tp_id = self.broker.next_order_id();
        let sl_id = self.broker.next_order_id();

        let entry = Order::new_limit(entry_id, OrderSide::Buy, size, entry_price);
        let mut tp = Order::new_limit(tp_id, OrderSide::Sell, size, take_profit);
        tp.parent_id = Some(entry_id);
        tp.oco_group = Some(entry_id);
        let mut sl = Order::new_stop(sl_id, OrderSide::Sell, size, stop_loss);
        sl.parent_id = Some(entry_id);
        sl.oco_group = Some(entry_id);

        self.broker.submit_order(entry, data_idx);
        self.broker.submit_order(tp, data_idx);
        self.broker.submit_order(sl, data_idx);

        (entry_id, tp_id, sl_id)
    }

    /// 提交 Bracket 卖出单：限价入场 + 止盈 + 止损
    /// 返回 (entry_id, take_profit_id, stop_loss_id)
    pub fn bracket_sell(&mut self, data_idx: usize, size: i64, entry_price: f64, take_profit: f64, stop_loss: f64) -> (u64, u64, u64) {
        let entry_id = self.broker.next_order_id();
        let tp_id = self.broker.next_order_id();
        let sl_id = self.broker.next_order_id();

        let entry = Order::new_limit(entry_id, OrderSide::Sell, size, entry_price);
        let mut tp = Order::new_limit(tp_id, OrderSide::Buy, size, take_profit);
        tp.parent_id = Some(entry_id);
        tp.oco_group = Some(entry_id);
        let mut sl = Order::new_stop(sl_id, OrderSide::Buy, size, stop_loss);
        sl.parent_id = Some(entry_id);
        sl.oco_group = Some(entry_id);

        self.broker.submit_order(entry, data_idx);
        self.broker.submit_order(tp, data_idx);
        self.broker.submit_order(sl, data_idx);

        (entry_id, tp_id, sl_id)
    }
}
