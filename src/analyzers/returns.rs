// TimeReturn 分析器：跟踪每根 bar 的组合收益率变化

use crate::analyzers::{AnalysisResult, Analyzer};
use crate::core::Bar;

/// TimeReturn 分析器：计算每根 bar 的周期收益率和汇总统计
///
/// 每根 bar 记录组合价值的变化率：(current - prev) / prev
/// 回测结束时输出总收益率、平均收益率、最大/最小单周期收益率。
pub struct TimeReturn {
    /// 所有周期收益率序列
    returns: Vec<f64>,
    /// 上一根 bar 的组合价值
    prev_value: Option<f64>,
    /// 初始组合价值（第一根 bar 的价值）
    first_value: Option<f64>,
}

impl TimeReturn {
    /// 创建 TimeReturn 分析器
    pub fn new() -> Self {
        Self {
            returns: Vec::new(),
            prev_value: None,
            first_value: None,
        }
    }
}

impl Default for TimeReturn {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TimeReturn {
    fn name(&self) -> &str {
        "TimeReturn"
    }

    fn next_bar(&mut self, _bar: &Bar, portfolio_value: f64, _cash: f64) {
        if let Some(prev) = self.prev_value {
            if prev > 0.0 {
                let ret = (portfolio_value - prev) / prev;
                self.returns.push(ret);
            }
        } else {
            // 第一根 bar，记录初始价值
            self.first_value = Some(portfolio_value);
        }
        self.prev_value = Some(portfolio_value);
    }

    fn stop(&mut self) -> AnalysisResult {
        let mut result = AnalysisResult::new(self.name());

        let n = self.returns.len() as f64;

        // 总收益率：(最终价值 - 初始价值) / 初始价值
        let total_return = match (self.prev_value, self.first_value) {
            (Some(last), Some(first)) if first > 0.0 => (last - first) / first,
            _ => 0.0,
        };
        result.set("total_return", total_return);

        if !self.returns.is_empty() {
            // 平均周期收益率
            let sum: f64 = self.returns.iter().sum();
            let avg = sum / n;
            result.set("avg_return", avg);

            // 最大单周期收益
            let max_ret = self
                .returns
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            result.set("max_return", max_ret);

            // 最大单周期亏损（最小收益率）
            let min_ret = self.returns.iter().cloned().fold(f64::INFINITY, f64::min);
            result.set("min_return", min_ret);
        } else {
            result.set("avg_return", 0.0);
            result.set("max_return", 0.0);
            result.set("min_return", 0.0);
        }

        result.set("returns_count", n);
        result
    }
}
