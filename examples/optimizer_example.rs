/// 参数优化示例
/// 使用 Optimizer 并行搜索 SMA 交叉策略的最优参数组合

use backtrader_rust::engine::Optimizer;
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

/// SMA 交叉策略（参数可配置）
struct SmaCrossOptimized {
    fast_period: usize,
    slow_period: usize,
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl SmaCrossOptimized {
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

impl Strategy for SmaCrossOptimized {
    fn init(&mut self, _ctx: &mut Context) {
        self.fast_sma = Some(SMA::new(self.fast_period));
        self.slow_sma = Some(SMA::new(self.slow_period));
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() {
            return;
        }
        let bar = &data[0isize];
        let close = bar.close;

        // 更新指标
        let fast_val = self.fast_sma.as_mut().unwrap().next(close);
        let slow_val = self.slow_sma.as_mut().unwrap().next(close);

        let (Some(fast), Some(slow)) = (fast_val, slow_val) else {
            return;
        };

        let pos_size = ctx.position(0).size;

        if let (Some(prev_f), Some(prev_s)) = (self.prev_fast, self.prev_slow) {
            // 金叉买入
            if prev_f <= prev_s && fast > slow && pos_size == 0 {
                let cash_available = ctx.cash() * 0.95;
                let size = (cash_available / close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                }
            }
            // 死叉卖出
            else if prev_f >= prev_s && fast < slow && pos_size > 0 {
                ctx.sell(0, pos_size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }
}

fn main() {
    println!("=== SMA 交叉策略参数优化示例 ===\n");

    // 创建优化器
    let optimizer = Optimizer::new(10000.0, "sample_data/orcl-2014.txt")
        .commission(0.005);

    // 生成参数空间：fast_period x slow_period（约束 fast < slow）
    let fast_periods = [5, 10, 15, 20];
    let slow_periods = [20, 30, 40, 50];

    let mut param_sets = Vec::new();
    for &fast in &fast_periods {
        for &slow in &slow_periods {
            if fast < slow {
                param_sets.push(vec![fast as f64, slow as f64]);
            }
        }
    }

    println!("参数组合数量: {}", param_sets.len());
    println!("开始并行优化...\n");

    // 运行优化，按最终组合价值降序排列
    let results = optimizer.run_sorted::<SmaCrossOptimized, _>(
        |params| {
            let fast = params[0] as usize;
            let slow = params[1] as usize;
            SmaCrossOptimized::new(fast, slow)
        },
        param_sets,
        |result| result.final_value,
        false, // 降序：最高价值在前
    );

    // 打印结果
    println!("==============================");
    println!("  参数优化结果（按最终价值排序）");
    println!("==============================");
    println!("{:>8} {:>8} {:>14} {:>10} {:>8}",
             "Fast", "Slow", "最终价值", "收益率%", "交易数");
    println!("----------------------------------------------");

    for opt_result in &results {
        let fast = opt_result.params[0] as usize;
        let slow = opt_result.params[1] as usize;
        println!(
            "{:>8} {:>8} {:>14.2} {:>10.2} {:>8}",
            fast,
            slow,
            opt_result.result.final_value,
            opt_result.result.total_return,
            opt_result.result.trades.len(),
        );
    }

    // 输出最优参数
    if let Some(best) = results.first() {
        println!("\n最优参数:");
        println!("  Fast SMA: {}", best.params[0] as usize);
        println!("  Slow SMA: {}", best.params[1] as usize);
        println!("  最终价值: {:.2}", best.result.final_value);
        println!("  总收益率: {:.2}%", best.result.total_return);
    }
}
