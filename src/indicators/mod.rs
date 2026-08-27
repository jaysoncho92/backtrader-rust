// Indicators 模块：技术指标框架

mod r#trait;
mod sma;
mod ema;
mod rsi;
mod macd;
mod bollinger;
mod atr;
mod stochastic;
mod crossover;

pub use r#trait::{Indicator, ChainedIndicator};
pub use sma::SMA;
pub use ema::EMA;
pub use rsi::RSI;
pub use macd::MACD;
pub use bollinger::BollingerBands;
pub use atr::ATR;
pub use stochastic::Stochastic;
pub use crossover::{CrossOver, CrossSignal};
