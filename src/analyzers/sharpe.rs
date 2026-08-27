// SharpeRatio 分析器：计算夏普比率，衡量风险调整后收益

use crate::analyzers::{AnalysisResult, Analyzer};
use crate::core::Bar;

/// SharpeRatio 分析器：基于每根 bar 的收益率计算夏普比率
///
/// 公式：Sharpe = (mean_return - rf_per_period) / std_dev * sqrt(annualization_factor)
/// 其中 rf_per_period = risk_free_rate / annualization_factor
pub struct SharpeRatio {
    /// 无风险年化利率（默认 0.0）
    risk_free_rate: f64,
    /// 年化因子（默认 252.0，即交易天数）
    annualization_factor: f64,
    /// 每根 bar 的收益率序列
    returns: Vec<f64>,
    /// 上一根 bar 的组合价值
    prev_value: Option<f64>,
}

impl SharpeRatio {
    /// 创建 SharpeRatio 分析器（使用默认参数）
    pub fn new() -> Self {
        Self {
            risk_free_rate: 0.0,
            annualization_factor: 252.0,
            returns: Vec::new(),
            prev_value: None,
        }
    }

    /// 创建 SharpeRatio 分析器（指定参数）
    pub fn with_params(risk_free_rate: f64, annualization_factor: f64) -> Self {
        Self {
            risk_free_rate,
            annualization_factor,
            returns: Vec::new(),
            prev_value: None,
        }
    }
}

impl Default for SharpeRatio {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SharpeRatio {
    fn name(&self) -> &str {
        "SharpeRatio"
    }

    fn next_bar(&mut self, _bar: &Bar, portfolio_value: f64, _cash: f64) {
        if let Some(prev) = self.prev_value {
            if prev > 0.0 {
                let ret = (portfolio_value - prev) / prev;
                self.returns.push(ret);
            }
        }
        self.prev_value = Some(portfolio_value);
    }

    fn stop(&mut self) -> AnalysisResult {
        let mut result = AnalysisResult::new(self.name());

        if self.returns.is_empty() {
            result.set("sharpe_ratio", 0.0);
            result.set("mean_return", 0.0);
            result.set("std_dev", 0.0);
            return result;
        }

        let n = self.returns.len() as f64;

        // 平均收益率
        let mean: f64 = self.returns.iter().sum::<f64>() / n;

        // 收益率标准差（总体标准差）
        let variance = self.returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        // 每期无风险利率
        let rf_per_period = self.risk_free_rate / self.annualization_factor;

        // 夏普比率
        let sharpe = if std_dev > 0.0 {
            (mean - rf_per_period) / std_dev * self.annualization_factor.sqrt()
        } else {
            0.0
        };

        result.set("sharpe_ratio", sharpe);
        result.set("mean_return", mean);
        result.set("std_dev", std_dev);
        result
    }
}
