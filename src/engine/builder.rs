use crate::analyzers::Analyzer;
use crate::brokers::{CommissionInfo, DefaultBroker, Slippage};
use crate::engine::cerebro::Cerebro;
use crate::feeds::DataFeed;
use crate::observers::Observer;
use crate::sizers::Sizer;
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
/// //     .add_analyzer(Box::new(TimeReturn::new()))
/// //     .add_observer(Box::new(BrokerValue::new()))
/// //     .sizer(Box::new(FixedSizer::new(100)))
/// //     .run()
/// ```
pub struct CerebroBuilder {
    cash: f64,
    commission: f64,
    feeds: Vec<Box<dyn DataFeed>>,
    strategy: Option<Box<dyn Strategy>>,
    /// 分析器列表（Phase 4）
    analyzers: Vec<Box<dyn Analyzer>>,
    /// 观察者列表（Phase 4）
    observers: Vec<Box<dyn Observer>>,
    /// Sizer（Phase 5）
    sizer: Option<Box<dyn Sizer>>,
    /// 修复 #5: 高级佣金配置（可选，覆盖简单百分比佣金）
    commission_info: Option<CommissionInfo>,
    /// 修复 #5: 滑点配置
    slippage: Option<Slippage>,
}

impl CerebroBuilder {
    pub fn new() -> Self {
        Self {
            cash: 10000.0,
            commission: 0.005,
            feeds: Vec::new(),
            strategy: None,
            analyzers: Vec::new(),
            observers: Vec::new(),
            sizer: None,
            commission_info: None,
            slippage: None,
        }
    }

    /// 设置初始资金
    pub fn cash(mut self, cash: f64) -> Self {
        self.cash = cash;
        self
    }

    /// 设置手续费率（简单百分比，向后兼容）
    /// 如需高级配置请使用 `commission_info()` 方法
    pub fn commission(mut self, rate: f64) -> Self {
        self.commission = rate;
        self
    }

    /// 修复 #5: 设置高级佣金配置（覆盖 commission() 的简单百分比设置）
    pub fn commission_info(mut self, info: CommissionInfo) -> Self {
        self.commission_info = Some(info);
        self
    }

    /// 修复 #5: 设置滑点模型
    pub fn slippage(mut self, slippage: Slippage) -> Self {
        self.slippage = Some(slippage);
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

    /// 添加分析器（Phase 4）
    pub fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }

    /// 添加观察者（Phase 4）
    pub fn add_observer(mut self, observer: Box<dyn Observer>) -> Self {
        self.observers.push(observer);
        self
    }

    /// 设置 Sizer（Phase 5）
    pub fn sizer(mut self, sizer: Box<dyn Sizer>) -> Self {
        self.sizer = Some(sizer);
        self
    }

    /// 构建并运行回测
    pub fn run(self) -> crate::engine::BacktestResult {
        let strategy = self.strategy.expect("必须添加 Strategy");
        // 修复 #5: 优先使用高级佣金配置，否则使用简单百分比佣金（向后兼容）
        let commission_info = self.commission_info
            .unwrap_or_else(|| CommissionInfo::new(self.commission));
        let mut broker = DefaultBroker::new(self.cash, commission_info);

        // 修复 #5: 应用滑点配置
        if let Some(slippage) = self.slippage {
            broker.set_slippage(slippage);
        }

        let mut cerebro = Cerebro::new(
            strategy,
            Box::new(broker),
            self.feeds,
            self.cash,
        );

        // 添加分析器和观察者（Phase 4）
        for analyzer in self.analyzers {
            cerebro.add_analyzer(analyzer);
        }
        for observer in self.observers {
            cerebro.add_observer(observer);
        }

        // 设置 Sizer（Phase 5）
        if let Some(sizer) = self.sizer {
            cerebro.set_sizer(sizer);
        }

        cerebro.run()
    }
}

impl Default for CerebroBuilder {
    fn default() -> Self {
        Self::new()
    }
}
