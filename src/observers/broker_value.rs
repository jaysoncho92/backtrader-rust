// BrokerValue Observer：记录每根 bar 的组合价值和现金

use crate::core::Bar;
use crate::observers::Observer;

/// BrokerValue Observer：记录每根 bar 的组合价值（现金 + 持仓市值）和现金
///
/// 数据结构：Vec<(bar_index, portfolio_value, cash)>
/// 可用于绘制资金曲线、分析资金变化趋势。
pub struct BrokerValue {
    /// 每根 bar 的记录：(bar_index, portfolio_value, cash)
    records: Vec<(usize, f64, f64)>,
}

impl BrokerValue {
    /// 创建 BrokerValue Observer
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// 获取所有记录：(bar_index, portfolio_value, cash)
    pub fn values(&self) -> &[(usize, f64, f64)] {
        &self.records
    }

    /// 获取最终组合价值（最后一根 bar 的价值）
    pub fn final_value(&self) -> Option<f64> {
        self.records.last().map(|(_, v, _)| *v)
    }

    /// 获取最终现金（最后一根 bar 的现金）
    pub fn final_cash(&self) -> Option<f64> {
        self.records.last().map(|(_, _, c)| *c)
    }

    /// 获取记录数量
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Default for BrokerValue {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for BrokerValue {
    fn name(&self) -> &str {
        "BrokerValue"
    }

    fn next(&mut self, bar_index: usize, _bar: &Bar, portfolio_value: f64, cash: f64) {
        self.records.push((bar_index, portfolio_value, cash));
    }
}
