// SQN 分析器：System Quality Number，衡量交易系统质量

use crate::analyzers::{AnalysisResult, Analyzer};
use crate::brokers::Trade;

/// SQN (System Quality Number) 分析器
///
/// 公式：SQN = sqrt(N) * mean(pnl) / std_dev(pnl)
/// 其中 N = 交易次数，pnl = 每笔交易的盈亏。
/// SQN > 3 通常表示优秀系统，SQN > 5 表示极好系统。
pub struct SQN {
    /// 已关闭交易的盈亏列表
    pnls: Vec<f64>,
}

impl SQN {
    /// 创建 SQN 分析器
    pub fn new() -> Self {
        Self { pnls: Vec::new() }
    }
}

impl Default for SQN {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SQN {
    fn name(&self) -> &str {
        "SQN"
    }

    fn on_trade(&mut self, trade: &Trade) {
        if trade.is_closed() {
            self.pnls.push(trade.pnl);
        }
    }

    fn stop(&mut self) -> AnalysisResult {
        let mut result = AnalysisResult::new(self.name());

        let n = self.pnls.len();
        result.set("trades", n as f64);

        if n == 0 {
            result.set("sqn", 0.0);
            result.set("expectancy", 0.0);
            return result;
        }

        let n_f64 = n as f64;

        // 期望值（平均每笔盈亏）
        let mean: f64 = self.pnls.iter().sum::<f64>() / n_f64;
        result.set("expectancy", mean);

        // 标准差
        let variance = self.pnls.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n_f64;
        let std_dev = variance.sqrt();

        // SQN = sqrt(N) * mean / std_dev
        let sqn = if std_dev > 0.0 {
            n_f64.sqrt() * mean / std_dev
        } else {
            // 所有交易盈亏完全相同，标准差为 0
            if mean > 0.0 {
                f64::INFINITY
            } else if mean < 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        };

        result.set("sqn", sqn);
        result
    }
}
