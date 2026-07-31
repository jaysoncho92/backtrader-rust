/// SMA 交叉策略完整示例
/// 使用 SMA(10) 和 SMA(30) 的金叉/死叉信号进行交易

use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::CsvFeed;
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

/// SMA 交叉策略
/// - fast_sma 上穿 slow_sma 时买入
/// - fast_sma 下穿 slow_sma 时卖出
struct SmaCrossStrategy {
    fast_period: usize,
    slow_period: usize,
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl SmaCrossStrategy {
    fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            fast_sma: None,
            slow_sma: None,
            prev_fast: None,
            prev_slow: None,
        }
    }
}

impl Strategy for SmaCrossStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.fast_sma = Some(SMA::new(self.fast_period));
        self.slow_sma = Some(SMA::new(self.slow_period));
        println!("策略初始化完成: SMA({}) x SMA({})", self.fast_period, self.slow_period);
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() {
            return;
        }

        let bar = &data[0isize]; // 最新 bar
        let close = bar.close;

        // 更新指标
        let fast_val = self.fast_sma.as_mut().unwrap().next(close);
        let slow_val = self.slow_sma.as_mut().unwrap().next(close);

        // 两个指标都必须就绪才能产生信号
        let (Some(fast), Some(slow)) = (fast_val, slow_val) else {
            return;
        };

        let pos = ctx.position(0);

        // 金叉：fast 上穿 slow，且无持仓 -> 买入
        if let (Some(prev_f), Some(prev_s)) = (self.prev_fast, self.prev_slow) {
            if prev_f <= prev_s && fast > slow && !pos.is_open() {
                // 计算可买数量：用 95% 的可用现金
                let cash_available = ctx.cash() * 0.95;
                let size = (cash_available / close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                    println!("[买入] bar={} dt={} close={:.2} fast={:.2} slow={:.2} size={}",
                             ctx.current_bar, bar.datetime, close, fast, slow, size);
                }
            }
            // 死叉：fast 下穿 slow，且有持仓 -> 卖出
            else if prev_f >= prev_s && fast < slow && pos.is_open() {
                let sell_size = pos.size;
                ctx.sell(0, sell_size);
                println!("[卖出] bar={} dt={} close={:.2} fast={:.2} slow={:.2} size={}",
                         ctx.current_bar, bar.datetime, close, fast, slow, sell_size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }

    fn stop(&mut self, ctx: &mut Context) {
        println!("\n回测结束!");
        println!("最终现金: {:.2}", ctx.cash());
        println!("最终组合价值: {:.2}", ctx.portfolio_value(0));
    }
}

fn main() {
    println!("=== Backtrader-Rust SMA 交叉策略示例 ===\n");

    // 创建 CSV 数据源
    let feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载 CSV 数据");
    println!("加载了 {} 根 K 线数据", feed.len());

    // 创建策略
    let strategy = SmaCrossStrategy::new(10, 30);

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
        println!("  交易 #{}: 入场={:.2} 出场={:.2} 盈亏={:.2} 手续费={:.2}",
                 i + 1, trade.entry_price, trade.exit_price, trade.pnl, trade.commission);
    }
}
