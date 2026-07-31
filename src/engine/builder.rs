use crate::brokers::{CommissionInfo, DefaultBroker};
use crate::engine::cerebro::Cerebro;
use crate::feeds::DataFeed;
use crate::strategy::Strategy;

/// CerebroBuilder：流畅 API 构建 Cerebro 实例
///
/// 使用示例：
/// ```no_run
/// use backtrader_rust::engine::CerebroBuilder;
/// // CerebroBuilder::new()
/// //     .cash(10000.0)
/// //     .commission(0.005)
/// //     .add_data(feed)
/// //     .add_strategy(strategy)
/// //     .run()
/// ```
pub struct CerebroBuilder {
    cash: f64,
    commission: f64,
    feeds: Vec<Box<dyn DataFeed>>,
    strategy: Option<Box<dyn Strategy>>,
}

impl CerebroBuilder {
    pub fn new() -> Self {
        Self {
            cash: 10000.0,
            commission: 0.005,
            feeds: Vec::new(),
            strategy: None,
        }
    }

    /// 设置初始资金
    pub fn cash(mut self, cash: f64) -> Self {
        self.cash = cash;
        self
    }

    /// 设置手续费率
    pub fn commission(mut self, rate: f64) -> Self {
        self.commission = rate;
        self
    }

    /// 添加数据源
    pub fn add_data(mut self, feed: Box<dyn DataFeed>) -> Self {
        self.feeds.push(feed);
        self
    }

    /// 添加策略
    pub fn add_strategy(mut self, strategy: Box<dyn Strategy>) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// 构建并运行回测
    pub fn run(self) -> crate::engine::BacktestResult {
        let strategy = self.strategy.expect("必须添加 Strategy");
        let commission_info = CommissionInfo::new(self.commission);
        let broker = DefaultBroker::new(self.cash, commission_info);

        let mut cerebro = Cerebro::new(
            strategy,
            Box::new(broker),
            self.feeds,
            self.cash,
        );
        cerebro.run()
    }
}

impl Default for CerebroBuilder {
    fn default() -> Self {
        Self::new()
    }
}
