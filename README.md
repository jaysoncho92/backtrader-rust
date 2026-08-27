# backtrader-rust

Python [backtrader](https://github.com/mementum/backtrader) 回测框架的 Rust 重写版。

借助 Rust 的**内存安全**保证、**零成本抽象**带来的**高性能**，以及基于 `rayon` 的**并发优化**能力，本项目在保持与 backtrader 一致的策略语义（数据源 / 策略 / Broker / 指标 / 分析器）的同时，将回测耗时降低一到两个数量级。

## 目录

- [性能基准对比](#性能基准对比)
- [核心功能特性](#核心功能特性)
- [快速开始](#快速开始)
- [项目结构](#项目结构)
- [测试](#测试)

## 性能基准对比

### 测试条件

- 相同数据文件、相同策略逻辑：SMA(10)/SMA(30) 交叉，市价单，初始资金 10,000，无手续费
- Python 侧使用 backtrader 1.9.76.123；Rust 侧为 release 模式
- 每项测试运行 5 次取平均

### 耗时对比

| 数据规模 | Python 总耗时 | Rust 总耗时 | 总加速比 | 纯回测加速比 |
| --- | --- | --- | --- | --- |
| 252 bars | 31.0 ms | 0.43 ms | ~73x | ~520x |
| 100,000 bars | 13.3 s | 100.5 ms | ~132x | ~692x |
| 1,000,000 bars | 94.8 s | 1.05 s | ~90x | ~344x |

### 吞吐量与内存

- **吞吐量**：Rust 约 **360 万 bars/s**，Python 约 **1 万 bars/s**
- **峰值内存**（100 万 bars）：Rust **~193 MB** vs Python **~608 MB**，内存节省约 **3.2 倍**

### 逻辑等价性验证（最终组合价值）

| 规模 | Python | Rust | 差异 |
| --- | --- | --- | --- |
| 252 | 9996.96 | 9996.96 | 完全一致 |
| 100,000 | 8370.05 | 8370.36 | 0.004% |
| 1,000,000 | 9810.67 | 9812.22 | 0.016% |

> 说明：微小差异来自浮点累积顺序等执行细节，逻辑等价性成立。

### 复现方式

```bash
# Rust 基准
cargo run --release --example benchmark

# Python 基准
python bench/python_bench.py
```

## 核心功能特性

- **策略定义**：`Strategy` trait + `Context`（`init` / `next` / `stop` 生命周期，下单与持仓查询均通过 `Context`）
- **数据层**：`CsvFeed` / `Resampler`（日 → 周 / 月线重采样）/ `MultiDataFeed`（多数据源并行回测）
- **Broker**：Market / Limit / Stop / StopLimit / OCO / Bracket 订单，4 种佣金类型，固定 / 百分比滑点
- **技术指标**：SMA / EMA / RSI / MACD / Bollinger / ATR / Stochastic / CrossOver，支持链式组合
- **分析器**：TimeReturn / SharpeRatio / DrawDown / TradeAnalyzer / SQN
- **观察者**：BrokerValue
- **Sizer**：Fixed / Percent / ATR
- **优化器**：基于 `rayon` 的并行参数搜索

## 快速开始

以下是一个最小可运行的 SMA 交叉策略示例（完整版见 [`examples/sma_crossover.rs`](examples/sma_crossover.rs)）：

```rust
use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::{CsvFeed, DataFeed};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

struct SmaCrossStrategy {
    fast_period: usize,
    slow_period: usize,
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl Strategy for SmaCrossStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.fast_sma = Some(SMA::new(self.fast_period));
        self.slow_sma = Some(SMA::new(self.slow_period));
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() {
            return;
        }
        let close = data[0isize].close;

        let fast_val = self.fast_sma.as_mut().unwrap().next(close);
        let slow_val = self.slow_sma.as_mut().unwrap().next(close);
        let (Some(fast), Some(slow)) = (fast_val, slow_val) else {
            return;
        };

        let pos_size = ctx.position(0).size;
        // 金叉且无持仓 -> 买入；死叉且有持仓 -> 卖出
        if let (Some(prev_f), Some(prev_s)) = (self.prev_fast, self.prev_slow) {
            if prev_f <= prev_s && fast > slow && pos_size == 0 {
                let size = (ctx.cash() * 0.95 / close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                }
            } else if prev_f >= prev_s && fast < slow && pos_size != 0 {
                ctx.sell(0, pos_size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }

    fn stop(&mut self, ctx: &mut Context) {
        println!("最终组合价值: {:.2}", ctx.portfolio_value(0));
    }
}

fn main() {
    let feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载 CSV 数据");

    let result = CerebroBuilder::new()
        .cash(10000.0)
        .commission(0.005)
        .add_data(Box::new(feed))
        .add_strategy(Box::new(SmaCrossStrategy {
            fast_period: 10,
            slow_period: 30,
            fast_sma: None,
            slow_sma: None,
            prev_fast: None,
            prev_slow: None,
        }))
        .run();

    println!("处理的 Bar 数: {}", result.bars_processed);
    println!("总收益率: {:.2}%", result.total_return);
}
```

运行示例：

```bash
cargo run --release --example sma_crossover
```

`examples/` 目录下还提供了更多示例：

| 示例 | 说明 |
| --- | --- |
| `sma_crossover.rs` | SMA(10)/SMA(30) 金叉死叉策略 |
| `rsi_strategy.rs` | RSI 超买超卖策略 |
| `stop_loss_strategy.rs` | 止损策略 |
| `multi_timeframe.rs` | 多周期重采样回测 |
| `optimizer_example.rs` | rayon 并行参数优化 |
| `benchmark.rs` | 性能基准测试 |

## 项目结构

```
backtrader-rust/
├── src/
│   ├── lib.rs          # 库入口
│   ├── core/           # Bar、时间序列、周期（Timeframe）等基础类型
│   ├── strategy/       # Strategy trait 与 Context
│   ├── feeds/          # CsvFeed / MultiDataFeed / Resampler
│   ├── indicators/     # SMA/EMA/RSI/MACD/Bollinger/ATR/Stochastic/CrossOver
│   ├── brokers/        # Broker、订单、仓位、佣金、滑点
│   ├── analyzers/      # TimeReturn/SharpeRatio/DrawDown/TradeAnalyzer/SQN
│   ├── observers/      # BrokerValue 观察者
│   ├── sizers/         # Fixed/Percent/ATR 仓位管理
│   └── engine/         # Cerebro 引擎、Builder、rayon 优化器
└── examples/           # 可运行示例（含性能基准）
```

## 测试

项目包含 **125 个测试**，覆盖引擎、Broker、指标、分析器等核心模块：

```bash
cargo test
```

基准测试可通过以下方式运行：

```bash
cargo run --release --example benchmark   # Rust
python bench/python_bench.py              # Python 对照
```
