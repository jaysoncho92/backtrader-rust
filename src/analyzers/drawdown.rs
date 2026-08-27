// DrawDown 分析器：跟踪组合价值的最大回撤和回撤持续时间

use crate::analyzers::{AnalysisResult, Analyzer};
use crate::core::Bar;

/// DrawDown 分析器：记录组合价值从峰值的回撤幅度
///
/// 输出最大回撤百分比、最大回撤金额、当前回撤、最长回撤持续 bar 数。
/// 回撤百分比为正值（例如 15.3 表示从峰值下跌 15.3%）。
pub struct DrawDown {
    /// 历史最高组合价值
    peak: f64,
    /// 最大回撤百分比（正值）
    max_drawdown: f64,
    /// 最大回撤金额（正值）
    max_drawdown_value: f64,
    /// 最大回撤发生时的峰值价值
    max_drawdown_peak: f64,
    /// 当前回撤百分比（正值）
    current_drawdown: f64,
    /// 当前连续处于回撤状态的 bar 数
    current_drawdown_bars: usize,
    /// 最长连续回撤 bar 数
    longest_drawdown_bars: usize,
    /// 是否已开始记录（第一根 bar 后）
    started: bool,
}

impl DrawDown {
    /// 创建 DrawDown 分析器
    pub fn new() -> Self {
        Self {
            peak: 0.0,
            max_drawdown: 0.0,
            max_drawdown_value: 0.0,
            max_drawdown_peak: 0.0,
            current_drawdown: 0.0,
            current_drawdown_bars: 0,
            longest_drawdown_bars: 0,
            started: false,
        }
    }
}

impl Default for DrawDown {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DrawDown {
    fn name(&self) -> &str {
        "DrawDown"
    }

    fn next_bar(&mut self, _bar: &Bar, portfolio_value: f64, _cash: f64) {
        if !self.started {
            // 第一根 bar：初始化峰值
            self.peak = portfolio_value;
            self.started = true;
            return;
        }

        // 更新历史峰值
        if portfolio_value > self.peak {
            self.peak = portfolio_value;
            // 新高：回撤重置
            self.current_drawdown = 0.0;
            self.current_drawdown_bars = 0;
        } else {
            // 当前价值低于峰值，计算回撤
            let dd_value = self.peak - portfolio_value;
            let dd_pct = if self.peak > 0.0 {
                dd_value / self.peak * 100.0
            } else {
                0.0
            };

            self.current_drawdown = dd_pct;
            self.current_drawdown_bars += 1;

            // 更新最长回撤持续 bar 数
            if self.current_drawdown_bars > self.longest_drawdown_bars {
                self.longest_drawdown_bars = self.current_drawdown_bars;
            }

            // 更新最大回撤
            if dd_pct > self.max_drawdown {
                self.max_drawdown = dd_pct;
                self.max_drawdown_value = dd_value;
                self.max_drawdown_peak = self.peak;
            }
        }
    }

    fn stop(&mut self) -> AnalysisResult {
        let mut result = AnalysisResult::new(self.name());

        result.set("max_drawdown", self.max_drawdown);
        result.set("max_drawdown_value", self.max_drawdown_value);
        result.set("max_drawdown_peak", self.max_drawdown_peak);
        result.set("current_drawdown", self.current_drawdown);
        result.set("longest_drawdown_bars", self.longest_drawdown_bars as f64);

        result
    }
}
