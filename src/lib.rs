// backtrader-rust: Rust 实现的量化回测框架
// Phase 1-5: 核心引擎 + 指标 + 分析器 + 观察者 + 高级功能

pub mod core;
pub mod feeds;
pub mod brokers;
pub mod strategy;
pub mod engine;
pub mod indicators;
pub mod analyzers;
pub mod observers;
pub mod sizers;
