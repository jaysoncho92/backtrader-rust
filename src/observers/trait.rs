// Observer trait：观察者的核心接口，用于实时跟踪回测过程

use crate::core::Bar;

/// Observer trait：在回测过程中观察和记录每根 bar 的状态
///
/// 观察者与分析器的区别：
/// - Observer 记录每根 bar 的详细数据（如组合价值序列）
/// - Analyzer 在回测结束时汇总计算绩效指标
pub trait Observer {
    /// 观察者名称
    fn name(&self) -> &str;

    /// 每根 bar 调用：记录该时刻的组合状态
    fn next(&mut self, bar_index: usize, bar: &Bar, portfolio_value: f64, cash: f64);

    /// 回测结束时调用（可选，默认空）
    fn stop(&mut self) {}
}
