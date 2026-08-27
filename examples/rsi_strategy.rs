/// RSI 策略示例
/// 当 RSI < oversold 时买入（超卖），RSI > overbought 时卖出（超买）

use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::{CsvFeed, DataFeed};
use backtrader_rust::indicators::{Indicator, RSI};
use backtrader_rust::strategy::{Context, Strategy};

/// RSI 均值回归策略
struct RsiStrategy {
    rsi_period: usize,
    oversold: f64,
    overbought: f64,
    rsi: Option<RSI>,
    trade_count: usize,
}

impl RsiStrategy {
    fn new(rsi_period: usize, oversold: f64, overbought: f64) -> Self {
        Self {
            rsi_period,
            oversold,
            overbought,
            rsi: None,
            trade_count: 0,
        }
    }
}

impl Strategy for RsiStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.rsi = Some(RSI::new(self.rsi_period));
        println!(
            "策略初始化: RSI({}), 超卖={}, 超买={}",
            self.rsi_period, self.oversold, self.overbought
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

        // 更新 RSI
        let rsi_val = match self.rsi.as_mut().unwrap().next(close) {
            Some(v) => v,
            None => return, // RSI 尚未就绪
        };

        let pos_size = ctx.position(0).size;
        let pos_open = pos_size != 0;

        if rsi_val < self.oversold && !pos_open {
            // RSI 超卖且无持仓 -> 买入
            let cash_available = ctx.cash() * 0.95;
            let size = (cash_available / close) as i64;
            if size > 0 {
                ctx.buy(0, size);
                self.trade_count += 1;
                println!(
                    "[买入] bar={} dt={} close={:.2} RSI={:.2} size={}",
                    current_bar, datetime, close, rsi_val, size
                );
            }
        } else if rsi_val > self.overbought && pos_open {
            // RSI 超买且有持仓 -> 卖出
            ctx.sell(0, pos_size);
            self.trade_count += 1;
            println!(
                "[卖出] bar={} dt={} close={:.2} RSI={:.2} size={}",
                current_bar, datetime, close, rsi_val, pos_size
            );
        }
    }

    fn stop(&mut self, ctx: &mut Context) {
        println!("\n回测结束!");
        println!("总交易次数: {}", self.trade_count);
        println!("最终现金: {:.2}", ctx.cash());
        println!("最终组合价值: {:.2}", ctx.portfolio_value(0));
    }
}

fn main() {
    println!("=== Backtrader-Rust RSI 策略示例 ===\n");

    // 创建 CSV 数据源
    let feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载 CSV 数据");
    println!("加载了 {} 根 K 线数据", feed.len());

    // 创建 RSI 策略
    let strategy = RsiStrategy::new(14, 30.0, 70.0);

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
