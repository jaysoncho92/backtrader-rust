// Analyzers 模块：回测分析器框架和各种绩效分析器实现

mod r#trait;
mod returns;
mod sharpe;
mod drawdown;
mod trade_analyzer;
mod sqn;

pub use r#trait::{Analyzer, AnalysisResult};
pub use returns::TimeReturn;
pub use sharpe::SharpeRatio;
pub use drawdown::DrawDown;
pub use trade_analyzer::TradeAnalyzer;
pub use sqn::SQN;
