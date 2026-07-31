use crate::brokers::{Broker, Trade};
use crate::core::{Bar, TimeSeries};
use crate::feeds::DataFeed;
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
}

/// Cerebro 主引擎：协调 Strategy、Broker、DataFeed 的运行
pub struct Cerebro {
    strategy: Box<dyn Strategy>,
    broker: Box<dyn Broker>,
    feeds: Vec<Box<dyn DataFeed>>,
    initial_cash: f64,
}

impl Cerebro {
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
        }
    }

    /// 运行回测主循环
    pub fn run(&mut self) -> BacktestResult {
        // 1. 为每个数据源创建 TimeSeries
        let mut data_series: Vec<TimeSeries<Bar>> = self.feeds.iter().map(|_| TimeSeries::new()).collect();

        // 2. 加载全部数据到 TimeSeries
        for (idx, feed) in self.feeds.iter_mut().enumerate() {
            feed.reset();
            while let Some(bar) = feed.next_bar() {
                data_series[idx].push(bar);
            }
        }

        let num_bars = if data_series.is_empty() { 0 } else { data_series[0].len() };

        // 3. 调用 strategy.init(ctx)
        {
            let empty_data: Vec<TimeSeries<Bar>> = data_series.iter().map(|_| TimeSeries::new()).collect();
            let mut ctx = Context::new(empty_data, &mut *self.broker as &mut dyn Broker, 0);
            self.strategy.init(&mut ctx);
        }

        // 4. 逐 bar 循环
        // 重新创建空的 TimeSeries 用于逐根推送
        let mut live_data: Vec<TimeSeries<Bar>> = self.feeds.iter().map(|_| TimeSeries::new()).collect();

        for bar_idx in 0..num_bars {
            // a. 将新 bar 推入 live_data，并通知 broker
            for (data_idx, series) in live_data.iter_mut().enumerate() {
                let bar = &data_series[data_idx];
                // 获取第 bar_idx 根 bar（通过 get 方法，ago 从最新开始算）
                let ago = (bar.len() - 1 - bar_idx) as isize;
                if let Some(b) = bar.get(ago) {
                    series.push(b.clone());
                    // b. broker.next_bar 处理挂单
                    self.broker.next_bar(b, data_idx);
                }
            }

            // c. 处理 broker notifications（通知 strategy）
            let _notifications = self.broker.drain_notifications();
            // Phase 1: 暂不将通知传给 strategy，Phase 2 增加 notify_order 回调

            // d. 调用 strategy.next 或 prenext
            // Phase 1 简化：所有指标由 strategy 自己在 next 中管理，引擎不检查指标就绪状态
            let mut ctx = Context::new(
                live_data.clone(),
                &mut *self.broker as &mut dyn Broker,
                bar_idx,
            );
            self.strategy.next(&mut ctx);

            // 将 strategy 可能修改的 live_data 同步回来（Context 拥有的是 clone，所以无法直接同步）
            // Phase 1 简化：strategy 只能通过 broker 操作数据，不直接修改 TimeSeries
        }

        // 5. 调用 strategy.stop(ctx)
        {
            let mut ctx = Context::new(
                live_data.clone(),
                &mut *self.broker as &mut dyn Broker,
                num_bars,
            );
            self.strategy.stop(&mut ctx);
        }

        // 6. 计算回测结果
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
        }
    }
}
