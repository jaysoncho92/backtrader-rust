use rayon::prelude::*;

use crate::brokers::{CommissionInfo, DefaultBroker};
use crate::engine::cerebro::{BacktestResult, Cerebro};
use crate::feeds::CsvFeed;
use crate::strategy::Strategy;

/// 参数优化结果
pub struct OptimizationResult {
    /// 参数组合
    pub params: Vec<f64>,
    /// 对应的回测结果
    pub result: BacktestResult,
}

/// 参数优化器：并行运行多组参数，寻找最优策略配置
pub struct Optimizer {
    cash: f64,
    commission: f64,
    data_path: String,
}

impl Optimizer {
    /// 创建优化器
    /// - cash: 初始资金
    /// - data_path: 数据文件路径
    pub fn new(cash: f64, data_path: &str) -> Self {
        Self {
            cash,
            commission: 0.005,
            data_path: data_path.to_string(),
        }
    }

    /// 设置手续费率
    pub fn commission(mut self, rate: f64) -> Self {
        self.commission = rate;
        self
    }

    /// 并行运行多组参数
    /// - strategy_factory: 根据参数创建策略实例的工厂函数
    /// - param_sets: 多组参数，每组是一个 Vec<f64>
    pub fn run<S, F>(
        &self,
        strategy_factory: F,
        param_sets: Vec<Vec<f64>>,
    ) -> Vec<OptimizationResult>
    where
        S: Strategy + Send + 'static,
        F: Fn(&[f64]) -> S + Sync,
    {
        param_sets
            .par_iter()
            .map(|params| {
                // 为每组参数创建独立的 Cerebro 实例
                let feed = CsvFeed::new(&self.data_path)
                    .expect("优化器无法加载数据文件");

                let strategy = strategy_factory(params);
                let commission_info = CommissionInfo::new(self.commission);
                let broker = DefaultBroker::new(self.cash, commission_info);

                let mut cerebro = Cerebro::new(
                    Box::new(strategy),
                    Box::new(broker),
                    vec![Box::new(feed)],
                    self.cash,
                );

                let result = cerebro.run();

                OptimizationResult {
                    params: params.clone(),
                    result,
                }
            })
            .collect()
    }

    /// 运行并返回按指定指标排序的结果
    /// - sort_by: 从 BacktestResult 提取排序值的函数
    /// - ascending: 是否升序排列
    pub fn run_sorted<S, F>(
        &self,
        strategy_factory: F,
        param_sets: Vec<Vec<f64>>,
        sort_by: fn(&BacktestResult) -> f64,
        ascending: bool,
    ) -> Vec<OptimizationResult>
    where
        S: Strategy + Send + 'static,
        F: Fn(&[f64]) -> S + Sync,
    {
        let mut results = self.run(strategy_factory, param_sets);

        if ascending {
            results.sort_by(|a, b| {
                sort_by(&a.result)
                    .partial_cmp(&sort_by(&b.result))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            results.sort_by(|a, b| {
                sort_by(&b.result)
                    .partial_cmp(&sort_by(&a.result))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        results
    }
}
