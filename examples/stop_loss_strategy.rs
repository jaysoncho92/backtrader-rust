/// 止损/止盈策略示例
/// 使用 SMA 作为入场信号，并设置固定百分比止损和止盈
///
/// 策略逻辑：
/// - 当收盘价 > SMA(20) 且无持仓 -> 买入
/// - 持有期间：
///   - 如果当前价 < 入场价 * (1 - stop_pct) -> 止损卖出
///   - 如果当前价 > 入场价 * (1 + profit_pct) -> 止盈卖出

use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::{CsvFeed, DataFeed};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

/// 止损止盈策略
struct StopLossStrategy {
    /// SMA 周期
    sma_period: usize,
    /// 止损百分比（如 0.02 = 2%）
    stop_pct: f64,
    /// 止盈百分比（如 0.05 = 5%）
    profit_pct: f64,
    /// SMA 指标
    sma: Option<SMA>,
    /// 入场价格记录
    entry_price: Option<f64>,
    /// 交易统计
    total_trades: usize,
    winning_trades: usize,
    losing_trades: usize,
}

impl StopLossStrategy {
    fn new(sma_period: usize, stop_pct: f64, profit_pct: f64) -> Self {
        Self {
            sma_period,
            stop_pct,
            profit_pct,
            sma: None,
            entry_price: None,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
        }
    }
}

impl Strategy for StopLossStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.sma = Some(SMA::new(self.sma_period));
        println!(
            "策略初始化: SMA({}), 止损={:.1}%, 止盈={:.1}%",
            self.sma_period,
            self.stop_pct * 100.0,
            self.profit_pct * 100.0
        );
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() {
            return;
        }

        let bar = &data[0isize];
        let close = bar.close;
        let datetime = bar.datetime;
        let current_bar = ctx.current_bar;

        // 更新 SMA
        let sma_val = self.sma.as_mut().unwrap().next(close);
        let Some(sma) = sma_val else {
            return; // SMA 尚未就绪
        };

        let pos_size = ctx.position(0).size;
        let has_position = pos_size > 0;

        if !has_position && close > sma {
            // 无持仓且收盘价 > SMA -> 买入
            let cash_available = ctx.cash() * 0.95;
            let size = (cash_available / close) as i64;
            if size > 0 {
                ctx.buy(0, size);
                self.entry_price = Some(close);
                println!(
                    "[买入] bar={} dt={} close={:.2} SMA={:.2} size={} 止损={:.2} 止盈={:.2}",
                    current_bar,
                    datetime,
                    close,
                    sma,
                    size,
                    close * (1.0 - self.stop_pct),
                    close * (1.0 + self.profit_pct)
                );
            }
        } else if has_position {
            if let Some(entry) = self.entry_price {
                let stop_price = entry * (1.0 - self.stop_pct);
                let profit_price = entry * (1.0 + self.profit_pct);

                if close < stop_price {
                    // 触发止损
                    ctx.sell(0, pos_size);
                    self.total_trades += 1;
                    self.losing_trades += 1;
                    println!(
                        "[止损] bar={} dt={} close={:.2} 入场={:.2} 亏损={:.2}%",
                        current_bar,
                        datetime,
                        close,
                        entry,
                        (close - entry) / entry * 100.0
                    );
                    self.entry_price = None;
                } else if close > profit_price {
                    // 触发止盈
                    ctx.sell(0, pos_size);
                    self.total_trades += 1;
                    self.winning_trades += 1;
                    println!(
                        "[止盈] bar={} dt={} close={:.2} 入场={:.2} 盈利={:.2}%",
                        current_bar,
                        datetime,
                        close,
                        entry,
                        (close - entry) / entry * 100.0
                    );
                    self.entry_price = None;
                }
            }
        }
    }

    fn stop(&mut self, ctx: &mut Context) {
        println!("\n========== 回测统计 ==========");
        println!("总交易次数:   {}", self.total_trades);
        println!("盈利次数:     {}", self.winning_trades);
        println!("亏损次数:     {}", self.losing_trades);
        if self.total_trades > 0 {
            println!(
                "胜率:         {:.1}%",
                self.winning_trades as f64 / self.total_trades as f64 * 100.0
            );
        }
        println!("最终现金:     {:.2}", ctx.cash());
        println!("最终组合价值: {:.2}", ctx.portfolio_value(0));
    }
}

fn main() {
    println!("=== Backtrader-Rust 止损止盈策略示例 ===\n");

    // 加载 CSV 数据
    let feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载 CSV 数据");
    println!("加载了 {} 根 K 线数据", feed.len());

    // 创建策略
    let strategy = StopLossStrategy::new(20, 0.02, 0.05);

    // 构建并运行回测
    let result = CerebroBuilder::new()
        .cash(10000.0)
        .commission(0.005)
        .add_data(Box::new(feed))
        .add_strategy(Box::new(strategy))
        .run();

    // 打印结果
    println!("\n========== 回测结果 ==========");
    println!("处理的 Bar 数: {}", result.bars_processed);
    println!("最终组合价值: {:.2}", result.final_value);
    println!("总收益率:     {:.2}%", result.total_return);
    println!("交易次数:     {}", result.trades.len());
    for (i, trade) in result.trades.iter().enumerate() {
        println!(
            "  交易 #{}: 入场={:.2} 出场={:.2} 盈亏={:.2} 手续费={:.2}",
            i + 1,
            trade.entry_price,
            trade.exit_price,
            trade.pnl,
            trade.commission
        );
    }
}
