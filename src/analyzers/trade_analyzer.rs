// TradeAnalyzer 分析器：统计所有已关闭交易的盈亏分布

use crate::analyzers::{AnalysisResult, Analyzer};
use crate::brokers::Trade;

/// TradeAnalyzer 分析器：统计已关闭交易的完整交易绩效
///
/// 输出：总交易数、胜率、平均盈利/亏损、最大盈利/亏损、盈利因子、总盈亏。
pub struct TradeAnalyzer {
    /// 已关闭交易的盈亏列表（已扣除手续费）
    pnls: Vec<f64>,
}

impl TradeAnalyzer {
    /// 创建 TradeAnalyzer 分析器
    pub fn new() -> Self {
        Self { pnls: Vec::new() }
    }
}

impl Default for TradeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TradeAnalyzer {
    fn name(&self) -> &str {
        "TradeAnalyzer"
    }

    fn on_trade(&mut self, trade: &Trade) {
        // 只记录已关闭的交易
        if trade.is_closed() {
            self.pnls.push(trade.pnl);
        }
    }

    fn stop(&mut self) -> AnalysisResult {
        let mut result = AnalysisResult::new(self.name());

        let total = self.pnls.len() as f64;
        result.set("total_trades", total);

        if self.pnls.is_empty() {
            result.set("won", 0.0);
            result.set("lost", 0.0);
            result.set("win_rate", 0.0);
            result.set("avg_win", 0.0);
            result.set("avg_loss", 0.0);
            result.set("max_win", 0.0);
            result.set("max_loss", 0.0);
            result.set("profit_factor", 0.0);
            result.set("total_pnl", 0.0);
            result.set("avg_pnl", 0.0);
            return result;
        }

        // 分类统计盈利和亏损交易
        let wins: Vec<f64> = self.pnls.iter().filter(|&&p| p > 0.0).cloned().collect();
        let losses: Vec<f64> = self.pnls.iter().filter(|&&p| p < 0.0).cloned().collect();

        let won = wins.len() as f64;
        let lost = losses.len() as f64;

        // 胜率（百分比）
        let win_rate = won / total * 100.0;

        // 平均盈利
        let avg_win = if !wins.is_empty() {
            wins.iter().sum::<f64>() / wins.len() as f64
        } else {
            0.0
        };

        // 平均亏损（负值）
        let avg_loss = if !losses.is_empty() {
            losses.iter().sum::<f64>() / losses.len() as f64
        } else {
            0.0
        };

        // 最大单笔盈利
        let max_win = wins.iter().cloned().fold(0.0_f64, f64::max);

        // 最大单笔亏损（负值，取最小）
        let max_loss = losses.iter().cloned().fold(0.0_f64, f64::min);

        // 总盈利和总亏损（绝对值）
        let total_wins: f64 = wins.iter().sum();
        let total_losses_abs: f64 = losses.iter().map(|x| x.abs()).sum();

        // 盈利因子 = 总盈利 / 总亏损
        let profit_factor = if total_losses_abs > 0.0 {
            total_wins / total_losses_abs
        } else if total_wins > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        // 总盈亏
        let total_pnl: f64 = self.pnls.iter().sum();

        // 平均盈亏
        let avg_pnl = total_pnl / total;

        result.set("won", won);
        result.set("lost", lost);
        result.set("win_rate", win_rate);
        result.set("avg_win", avg_win);
        result.set("avg_loss", avg_loss);
        result.set("max_win", max_win);
        result.set("max_loss", max_loss);
        result.set("profit_factor", profit_factor);
        result.set("total_pnl", total_pnl);
        result.set("avg_pnl", avg_pnl);

        result
    }
}
