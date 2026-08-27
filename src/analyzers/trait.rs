// Analyzer trait 和 AnalysisResult：分析器框架的核心接口

use std::collections::HashMap;

use crate::brokers::Trade;
use crate::core::Bar;

/// 分析结果：存储分析器的输出指标
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// 分析器名称
    pub name: String,
    /// 指标键值对（如 "total_return" => 0.15）
    pub values: HashMap<String, f64>,
}

impl AnalysisResult {
    /// 创建空的分析结果
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    /// 设置指标值
    pub fn set(&mut self, key: &str, value: f64) {
        self.values.insert(key.to_string(), value);
    }

    /// 获取指标值
    pub fn get(&self, key: &str) -> Option<f64> {
        self.values.get(key).copied()
    }

    /// 格式化打印分析结果摘要
    pub fn print_summary(&self) {
        println!("=== {} ===", self.name);
        // 收集并排序键名，使输出稳定
        let mut keys: Vec<&String> = self.values.keys().collect();
        keys.sort();
        for key in keys {
            let val = self.values[key];
            // 根据数值大小选择显示精度
            if val.abs() > 1000.0 {
                println!("  {}: {:.2}", key, val);
            } else if val.abs() < 0.01 && val != 0.0 {
                println!("  {}: {:.6}", key, val);
            } else {
                println!("  {}: {:.4}", key, val);
            }
        }
    }
}

/// 分析器 trait：定义分析器的生命周期回调
///
/// 分析器用于在回测过程中收集数据，并在回测结束时计算绩效指标。
pub trait Analyzer {
    /// 分析器名称
    fn name(&self) -> &str;

    /// 每根 bar 调用（可选），用于跟踪组合价值变化
    fn next_bar(&mut self, _bar: &Bar, _portfolio_value: f64, _cash: f64) {}

    /// 当交易关闭时调用（可选），用于统计交易信息
    fn on_trade(&mut self, _trade: &Trade) {}

    /// 回测结束时调用，返回分析结果
    fn stop(&mut self) -> AnalysisResult;
}
