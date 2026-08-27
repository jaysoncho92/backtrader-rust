//! 性能基准测试：backtrader-rust vs Python backtrader
//!
//! 标准基准策略：SMA(10)/SMA(30) 交叉，市价单，初始资金 10,000，
//! 默认仓位 1 手（对齐 Python backtrader 的默认 FixedSizer 行为），
//! 手续费 0（对齐 Python backtrader 默认配置）。
//!
//! 对每个数据规模（252 / 100,000 / 1,000,000）重复运行 5 次取平均，
//! 分别记录「数据加载」与「完整回测」耗时。
//!
//! 运行方式（必须 release 模式）：
//!   cargo run --release --example benchmark

use std::mem::size_of;
use std::time::Instant;

use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::{CsvFeed, DataFeed};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

const REPEATS: usize = 5;
const INITIAL_CASH: f64 = 10_000.0;

/// SMA 交叉策略（与 Python 端 SmaCross 完全对应）
/// - 无持仓且 crossover > 0（fast 上穿 slow）→ 市价买入 1 手
/// - 有持仓且 crossover < 0（fast 下穿 slow）→ 市价卖出平仓
struct SmaCross {
    fast_period: usize,
    slow_period: usize,
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl SmaCross {
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

impl Strategy for SmaCross {
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

        // 交叉检测（等价于 bt.ind.CrossOver 的符号判定）
        if let (Some(pf), Some(ps)) = (self.prev_fast, self.prev_slow) {
            if pf <= ps && fast > slow && pos_size == 0 {
                // 对齐 Python 默认 FixedSizer：买入 1 手
                ctx.buy(0, 1);
            } else if pf >= ps && fast < slow && pos_size > 0 {
                ctx.sell(0, pos_size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }
}

/// 一次完整测量：加载耗时 + 回测耗时 + 最终组合价值
struct RunStats {
    load_ms: f64,
    backtest_ms: f64,
    final_value: f64,
    bars: usize,
    trades: usize,
}

fn run_once(path: &str) -> RunStats {
    // 1. 数据加载计时（对应 Python 端 feed 构造计时）
    let t_load = Instant::now();
    let feed = CsvFeed::new(path).expect("无法加载 CSV 数据");
    let bars = feed.len();
    let load_ms = t_load.elapsed().as_secs_f64() * 1000.0;

    // 2. 完整回测计时（对应 Python 端 cerebro.run()）
    let t_run = Instant::now();
    let result = CerebroBuilder::new()
        .cash(INITIAL_CASH)
        .commission(0.0) // 对齐 Python backtrader 默认（无手续费）
        .add_data(Box::new(feed))
        .add_strategy(Box::new(SmaCross::new(10, 30)))
        .run();
    let backtest_ms = t_run.elapsed().as_secs_f64() * 1000.0;

    RunStats {
        load_ms,
        backtest_ms,
        final_value: result.final_value,
        bars,
        trades: result.trades.len(),
    }
}

fn bench(path: &str, label: &str) {
    let mut loads = Vec::with_capacity(REPEATS);
    let mut runs = Vec::with_capacity(REPEATS);
    let mut last = None;

    for _ in 0..REPEATS {
        let s = run_once(path);
        loads.push(s.load_ms);
        runs.push(s.backtest_ms);
        last = Some(s);
    }

    let avg_load = loads.iter().sum::<f64>() / loads.len() as f64;
    let avg_run = runs.iter().sum::<f64>() / runs.len() as f64;
    let s = last.unwrap();
    let total = avg_load + avg_run;
    let throughput = s.bars as f64 / (avg_run / 1000.0);

    println!("== {} | {} ==", label, path);
    println!("  bars           : {}", s.bars);
    println!("  加载耗时(平均) : {:.3} ms", avg_load);
    println!("  回测耗时(平均) : {:.3} ms", avg_run);
    println!("  总耗时(平均)   : {:.3} ms", total);
    println!("  回测吞吐量     : {:.0} bars/s", throughput);
    println!("  最终组合价值   : {:.2}", s.final_value);
    println!("  完成交易数     : {}", s.trades);
    println!();
}

/// Windows: 当前进程峰值工作集（MB）
#[cfg(windows)]
fn peak_working_set_mb() -> Option<f64> {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct ProcessMemoryCounters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }
    unsafe {
        let mut counters = std::mem::zeroed::<ProcessMemoryCounters>();
        counters.cb = size_of::<ProcessMemoryCounters>() as u32;
        extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(
                process: isize,
                counters: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }
        let ok = K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            counters.cb,
        );
        if ok != 0 {
            Some(counters.PeakWorkingSetSize as f64 / (1024.0 * 1024.0))
        } else {
            None
        }
    }
}

fn main() {
    println!("=== backtrader-rust 性能基准 (release, {} 次重复取平均) ===\n", REPEATS);
    bench("sample_data/orcl-2014.txt", "252");
    bench("sample_data/bench_100k.csv", "100000");
    bench("sample_data/bench_1m.csv", "1000000");

    #[cfg(windows)]
    if let Some(mb) = peak_working_set_mb() {
        println!("进程峰值内存(工作集): {:.1} MB", mb);
    }
}
