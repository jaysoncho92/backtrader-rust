use crate::analyzers::{AnalysisResult, Analyzer};
use crate::brokers::{Broker, Trade};
use crate::core::{Bar, TimeSeries};
use crate::feeds::DataFeed;
use crate::observers::Observer;
use crate::sizers::Sizer;
use crate::strategy::{Context, Strategy};

/// 回测结果结构
#[derive(Debug, Clone)]
pub struct BacktestResult {
    /// 最终组合价值
    pub final_value: f64,
    /// 总收益率（百分比）
    pub total_return: f64,
    /// 已完成的交易记录
    pub trades: Vec<Trade>,
    /// 处理的 bar 数量
    pub bars_processed: usize,
    /// 分析器结果（Phase 4 新增）
    pub analyzer_results: Vec<AnalysisResult>,
}

impl BacktestResult {
    /// 格式化打印所有分析器结果摘要
    pub fn print_summary(&self) {
        println!("==============================");
        println!("  回测结果摘要");
        println!("==============================");
        println!("  最终价值: {:.2}", self.final_value);
        println!("  总收益率: {:.2}%", self.total_return);
        println!("  处理 bar 数: {}", self.bars_processed);
        println!("  交易次数: {}", self.trades.len());
        println!("------------------------------");
        for ar in &self.analyzer_results {
            ar.print_summary();
            println!("------------------------------");
        }
    }
}

/// Cerebro 主引擎：协调 Strategy、Broker、DataFeed、Analyzer、Observer 的运行
pub struct Cerebro {
    strategy: Box<dyn Strategy>,
    broker: Box<dyn Broker>,
    feeds: Vec<Box<dyn DataFeed>>,
    initial_cash: f64,
    /// 分析器列表（Phase 4）
    analyzers: Vec<Box<dyn Analyzer>>,
    /// 观察者列表（Phase 4）
    observers: Vec<Box<dyn Observer>>,
    /// Sizer（Phase 5）
    sizer: Option<Box<dyn Sizer>>,
}

impl Cerebro {
    /// 创建 Cerebro 实例（保持向后兼容，分析器和观察者为空）
    pub fn new(
        strategy: Box<dyn Strategy>,
        broker: Box<dyn Broker>,
        feeds: Vec<Box<dyn DataFeed>>,
        initial_cash: f64,
    ) -> Self {
        Self {
            strategy,
            broker,
            feeds,
            initial_cash,
            analyzers: Vec::new(),
            observers: Vec::new(),
            sizer: None,
        }
    }

    /// 添加分析器（Phase 4）
    pub fn add_analyzer(&mut self, analyzer: Box<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }

    /// 添加观察者（Phase 4）
    pub fn add_observer(&mut self, observer: Box<dyn Observer>) {
        self.observers.push(observer);
    }

    /// 设置 Sizer（Phase 5）
    pub fn set_sizer(&mut self, sizer: Box<dyn Sizer>) {
        self.sizer = Some(sizer);
    }

    /// 运行回测主循环
    pub fn run(&mut self) -> BacktestResult {
        // 1. 为每个数据源创建 TimeSeries
        let mut data_series: Vec<TimeSeries<Bar>> =
            self.feeds.iter().map(|_| TimeSeries::new()).collect();

        // 2. 加载全部数据到 TimeSeries
        for (idx, feed) in self.feeds.iter_mut().enumerate() {
            feed.reset();
            while let Some(bar) = feed.next_bar() {
                data_series[idx].push(bar);
            }
        }

        let num_bars = if data_series.is_empty() {
            0
        } else {
            data_series[0].len()
        };

        // 3. 调用 strategy.init(ctx)
        // 修复 #3: 传引用，不再克隆空数据
        {
            let empty_data: Vec<TimeSeries<Bar>> =
                data_series.iter().map(|_| TimeSeries::new()).collect();
            let mut ctx = Context::with_sizer(
                &empty_data,
                &mut *self.broker as &mut dyn Broker,
                0,
                self.sizer.as_deref(),
            );
            self.strategy.init(&mut ctx);
        }

        // 4. 逐 bar 循环
        // 重新创建空的 TimeSeries 用于逐根推送
        let mut live_data: Vec<TimeSeries<Bar>> =
            self.feeds.iter().map(|_| TimeSeries::new()).collect();

        // 跟踪已扫描的交易总数（增量扫描，避免每根 bar 全量过滤导致 O(N²)）
        let mut scanned_trade_count: usize = 0;

        for bar_idx in 0..num_bars {
            // a. 将新 bar 推入 live_data，并通知 broker
            for (data_idx, series) in live_data.iter_mut().enumerate() {
                let bar = &data_series[data_idx];
                // 获取第 bar_idx 根 bar（通过 get 方法，ago 从最新开始算）
                // 对于多数据源，当 bar_idx 超过该数据源长度时跳过
                if bar_idx < bar.len() {
                    let ago = (bar.len() - 1 - bar_idx) as isize;
                    if let Some(b) = bar.get(ago) {
                        series.push(b.clone());
                        // b. broker.next_bar 处理挂单
                        self.broker.next_bar(b, data_idx);
                    }
                }
            }

            // c. 检查新关闭的交易，通知分析器（增量扫描，均摊 O(1)）
            let all_trades = self.broker.get_trades();
            for i in scanned_trade_count..all_trades.len() {
                let trade = &all_trades[i];
                if trade.is_closed() {
                    for analyzer in self.analyzers.iter_mut() {
                        analyzer.on_trade(trade);
                    }
                }
            }
            scanned_trade_count = all_trades.len();

            // d. 通知观察者和分析器（每根 bar）
            // 获取第一数据源的最新 bar 用于计算组合价值
            if let Some(bar) = live_data.first().and_then(|s| s.last()) {
                let portfolio_value = self.broker.get_value(bar, 0);
                let cash = self.broker.get_cash();

                for observer in self.observers.iter_mut() {
                    observer.next(bar_idx, bar, portfolio_value, cash);
                }
                for analyzer in self.analyzers.iter_mut() {
                    analyzer.next_bar(bar, portfolio_value, cash);
                }
            }

            // e. 处理 broker notifications
            let _notifications = self.broker.drain_notifications();

            // f. 调用 strategy.next 或 prenext
            // 修复 #3: 传 live_data 的引用，不再每根 bar 克隆整个 Vec → O(N²) → O(N)
            let mut ctx = Context::with_sizer(
                &live_data,
                &mut *self.broker as &mut dyn Broker,
                bar_idx,
                self.sizer.as_deref(),
            );
            self.strategy.next(&mut ctx);
        }

        // 5. 调用 strategy.stop(ctx)
        // 修复 #3: 传引用，不再克隆
        {
            let mut ctx = Context::with_sizer(
                &live_data,
                &mut *self.broker as &mut dyn Broker,
                num_bars,
                self.sizer.as_deref(),
            );
            self.strategy.stop(&mut ctx);
        }

        // 6. 回测结束：停止所有观察者和分析器
        for observer in self.observers.iter_mut() {
            observer.stop();
        }

        let analyzer_results: Vec<AnalysisResult> =
            self.analyzers.iter_mut().map(|a| a.stop()).collect();

        // 7. 计算回测结果
        let final_bar = live_data.first().and_then(|s| s.last());
        let final_value = if let Some(bar) = final_bar {
            self.broker.get_value(bar, 0)
        } else {
            self.broker.get_cash()
        };

        let total_return = (final_value - self.initial_cash) / self.initial_cash * 100.0;

        BacktestResult {
            final_value,
            total_return,
            trades: self.broker.get_trades().to_vec(),
            bars_processed: num_bars,
            analyzer_results,
        }
    }
}
