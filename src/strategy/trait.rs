use crate::core::{Bar, TimeSeries};
use crate::brokers::{Broker, Order, OrderSide, OrderType, Position};

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

/// Context：Strategy 访问数据和 Broker 的桥梁
/// 由 Cerebro 引擎在每根 bar 时构建并传给 Strategy
pub struct Context<'a> {
    /// 多数据源的时序数据
    pub data: Vec<TimeSeries<Bar>>,
    /// Broker 引用（用于下单、查询资金）
    pub broker: &'a mut dyn Broker,
    /// 当前处理到的 bar 索引（从 0 开始）
    pub current_bar: usize,
}

impl<'a> Context<'a> {
    pub fn new(
        data: Vec<TimeSeries<Bar>>,
        broker: &'a mut dyn Broker,
        current_bar: usize,
    ) -> Self {
        Self {
            data,
            broker,
            current_bar,
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
}
