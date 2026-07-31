// Engine 模块：Cerebro 回测引擎

mod cerebro;
mod builder;

pub use cerebro::{Cerebro, BacktestResult};
pub use builder::CerebroBuilder;
