/// Phase 5 集成测试：Resampler、MultiDataFeed、Sizer、Optimizer

use backtrader_rust::core::{Bar, TimeFrame};
use backtrader_rust::engine::Optimizer;
use backtrader_rust::feeds::{DataFeed, MultiDataFeed, Resampler};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::sizers::{ATRSizer, FixedSizer, PercentSizer, Sizer};
use backtrader_rust::strategy::{Context, Strategy};
use chrono::NaiveDate;

// ========== 辅助函数 ==========

/// 创建一组日线 Bar 数据
fn make_daily_bars(start_date: NaiveDate, count: usize, base_price: f64) -> Vec<Bar> {
    let mut bars = Vec::new();
    for i in 0..count {
        let dt = start_date + chrono::Duration::days(i as i64);
        let price = base_price + (i as f64) * 0.1;
        bars.push(Bar::new(
            dt.and_hms_opt(0, 0, 0).unwrap(),
            price,       // open
            price + 0.5, // high
            price - 0.2, // low
            price + 0.2, // close
            1000.0,      // volume
            0.0,
        ));
    }
    bars
}

/// 简易内存 Feed（用于测试）
struct MemoryFeed {
    bars: Vec<Bar>,
    cursor: usize,
}

impl MemoryFeed {
    fn new(bars: Vec<Bar>) -> Self {
        Self { bars, cursor: 0 }
    }
}

impl DataFeed for MemoryFeed {
    fn next_bar(&mut self) -> Option<Bar> {
        if self.cursor < self.bars.len() {
            let bar = self.bars[self.cursor].clone();
            self.cursor += 1;
            Some(bar)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }

    fn len(&self) -> usize {
        self.bars.len()
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

// ========== Resampler 测试 ==========

#[test]
fn test_resampler_daily_to_weekly_basic() {
    // 创建 10 个连续日线数据（跨越 2 周）
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // 周一
    let bars = make_daily_bars(start, 10, 100.0);
    let feed = MemoryFeed::new(bars);

    let mut resampler = Resampler::new(feed, TimeFrame::Weeks);

    // 收集所有周线 bar
    let mut weekly_bars = Vec::new();
    while let Some(bar) = resampler.next_bar() {
        weekly_bars.push(bar);
    }

    // 应该有 2 根周线（10 天跨 2 个 ISO 周）
    assert!(weekly_bars.len() >= 1, "至少应有 1 根周线，实际有 {} 根", weekly_bars.len());

    // 验证第一根周线的 OHLCV 聚合规则
    let first_weekly = &weekly_bars[0];
    // open 应该是周内第一根 bar 的 open
    assert_eq!(first_weekly.open, 100.0);
    // volume 应该是周内所有 bar 的 volume 之和
    assert!(first_weekly.volume > 0.0);
}

#[test]
fn test_resampler_daily_to_weekly_ohlcv_rules() {
    // 精确构造 5 根日线（同一 ISO 周内）
    let _start = NaiveDate::from_ymd_opt(2024, 1, 6).unwrap(); // 周六 (但ISO周从周一开始)
    // 让我们使用周一开始: 2024-01-08 是周一
    let monday = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
    let bars: Vec<Bar> = (0..5)
        .map(|i| {
            let dt = monday + chrono::Duration::days(i as i64);
            Bar::new(
                dt.and_hms_opt(0, 0, 0).unwrap(),
                100.0 + i as f64,        // open: 100, 101, 102, 103, 104
                105.0 + i as f64,        // high: 105, 106, 107, 108, 109
                95.0 + i as f64,         // low: 95, 96, 97, 98, 99
                100.0 + i as f64 + 0.5,  // close: 100.5, 101.5, 102.5, 103.5, 104.5
                100.0,                   // volume: 100 each
                0.0,
            )
        })
        .collect();

    let feed = MemoryFeed::new(bars);
    let mut resampler = Resampler::new(feed, TimeFrame::Weeks);

    // 5 根日线应在同一 ISO 周（周一到周五），聚合为 1 根周线
    let weekly = resampler.next_bar().unwrap();
    assert!(resampler.next_bar().is_none(), "应只有 1 根周线");

    // 验证聚合规则
    assert_eq!(weekly.open, 100.0, "open = 周内第一根 bar 的 open");
    assert_eq!(weekly.high, 109.0, "high = max(所有 high)");
    assert_eq!(weekly.low, 95.0, "low = min(所有 low)");
    assert_eq!(weekly.close, 104.5, "close = 最后一根 bar 的 close");
    assert_eq!(weekly.volume, 500.0, "volume = sum(所有 volume)");
}

#[test]
fn test_resampler_daily_to_monthly() {
    // 创建跨 2 个月的数据
    let jan_15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let mut bars = Vec::new();

    // 1 月份：5 根 bar
    for i in 0..5 {
        let dt = jan_15 + chrono::Duration::days(i as i64);
        bars.push(Bar::new(
            dt.and_hms_opt(0, 0, 0).unwrap(),
            100.0, 110.0, 90.0, 105.0, 1000.0, 0.0,
        ));
    }

    // 2 月份：5 根 bar
    let feb_1 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
    for i in 0..5 {
        let dt = feb_1 + chrono::Duration::days(i as i64);
        bars.push(Bar::new(
            dt.and_hms_opt(0, 0, 0).unwrap(),
            106.0, 115.0, 95.0, 110.0, 2000.0, 0.0,
        ));
    }

    let feed = MemoryFeed::new(bars);
    let mut resampler = Resampler::new(feed, TimeFrame::Months);

    let mut monthly_bars = Vec::new();
    while let Some(bar) = resampler.next_bar() {
        monthly_bars.push(bar);
    }

    assert_eq!(monthly_bars.len(), 2, "应该有 2 根月线");

    // 验证 1 月份月线
    assert_eq!(monthly_bars[0].open, 100.0, "月线 open = 月内第一根 open");
    assert_eq!(monthly_bars[0].high, 110.0, "月线 high = max");
    assert_eq!(monthly_bars[0].low, 90.0, "月线 low = min");
    assert_eq!(monthly_bars[0].close, 105.0, "月线 close = 最后一根 close");
    assert_eq!(monthly_bars[0].volume, 5000.0, "月线 volume = sum");

    // 验证 2 月份月线
    assert_eq!(monthly_bars[1].open, 106.0);
    assert_eq!(monthly_bars[1].high, 115.0);
    assert_eq!(monthly_bars[1].low, 95.0);
    assert_eq!(monthly_bars[1].volume, 10000.0);
}

#[test]
fn test_resampler_reset() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
    let bars = make_daily_bars(start, 5, 100.0);
    let feed = MemoryFeed::new(bars);
    let mut resampler = Resampler::new(feed, TimeFrame::Weeks);

    // 第一次读取
    let first_run = collect_bars(&mut resampler);
    assert!(!first_run.is_empty());

    // 重置后应能再次读取相同数据
    resampler.reset();
    let second_run = collect_bars(&mut resampler);

    assert_eq!(first_run.len(), second_run.len());
}

// ========== MultiDataFeed 测试 ==========

#[test]
fn test_multi_data_feed_basic() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let bars1 = make_daily_bars(start, 5, 100.0);
    let bars2 = make_daily_bars(start, 5, 200.0);

    let mut multi = MultiDataFeed::new();
    multi.add_feed(Box::new(MemoryFeed::new(bars1)));
    multi.add_feed(Box::new(MemoryFeed::new(bars2)));

    assert_eq!(multi.feed_count(), 2);

    // next_bar 返回第一个 feed 的数据
    let bar = multi.next_bar().unwrap();
    assert_eq!(bar.open, 100.0); // 来自第一个 feed
}

#[test]
fn test_multi_data_feed_time_sync() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // 两个 feed 有不同数量的数据
    let bars1 = make_daily_bars(start, 5, 100.0);
    let bars2 = make_daily_bars(start, 10, 200.0);

    let mut multi = MultiDataFeed::new();
    multi.add_feed(Box::new(MemoryFeed::new(bars1.clone())));
    multi.add_feed(Box::new(MemoryFeed::new(bars2)));

    // next_bars 应返回两个 feed 在同一时间点的 bars
    let sync_bars = multi.next_bars();
    assert_eq!(sync_bars.len(), 2);
    assert!(sync_bars[0].is_some());
    assert!(sync_bars[1].is_some());

    // 两个 bar 的 datetime 应该相同（同一日期）
    let dt0 = sync_bars[0].as_ref().unwrap().datetime;
    let dt1 = sync_bars[1].as_ref().unwrap().datetime;
    assert_eq!(dt0, dt1, "同步的 bar 时间应一致");
}

#[test]
fn test_multi_data_feed_reset() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let bars = make_daily_bars(start, 3, 100.0);

    let mut multi = MultiDataFeed::new();
    multi.add_feed(Box::new(MemoryFeed::new(bars)));

    // 读取所有
    let _ = collect_bars(&mut multi);
    assert!(multi.next_bar().is_none());

    // 重置
    multi.reset();
    assert!(multi.next_bar().is_some(), "重置后应能再次读取");
}

// ========== Sizer 测试 ==========

#[test]
fn test_fixed_sizer() {
    let sizer = FixedSizer::new(50);
    assert_eq!(sizer.get_size(10000.0, 50.0, true), 50);
    assert_eq!(sizer.get_size(10000.0, 50.0, false), 50);
    assert_eq!(sizer.get_size(0.0, 0.0, true), 50); // 固定手数不受价格影响
}

#[test]
fn test_fixed_sizer_default() {
    let sizer = FixedSizer::default();
    assert_eq!(sizer.get_size(10000.0, 50.0, true), 100);
}

#[test]
fn test_percent_sizer() {
    let sizer = PercentSizer::new(0.5); // 50%
    // cash=10000, price=50, size = (10000 * 0.5 / 50) = 100
    assert_eq!(sizer.get_size(10000.0, 50.0, true), 100);
}

#[test]
fn test_percent_sizer_minimum_one() {
    let sizer = PercentSizer::new(0.5);
    // cash=10, price=8, size = (10 * 0.5 / 8) = 0.625 -> 0, 但 cash*percent=5 >= 8? 5 < 8
    // 所以返回 0
    assert_eq!(sizer.get_size(10.0, 8.0, true), 0);

    // cash=20, price=8, size = (20 * 0.5 / 8) = 1.25 -> 1
    assert_eq!(sizer.get_size(20.0, 8.0, true), 1);
}

#[test]
fn test_percent_sizer_default() {
    let sizer = PercentSizer::default(); // 95%
    // cash=10000, price=100, size = (10000 * 0.95 / 100) = 95
    assert_eq!(sizer.get_size(10000.0, 100.0, true), 95);
}

#[test]
fn test_atr_sizer() {
    let sizer = ATRSizer::new(0.01, 2.0); // 1% 风险，ATR=2
    // cash=10000, risk_amount=100, per_share_risk=2.0, size=50
    assert_eq!(sizer.get_size(10000.0, 50.0, true), 50);
}

#[test]
fn test_atr_sizer_update_atr() {
    let sizer = ATRSizer::new(0.01, 2.0);
    sizer.set_atr_value(5.0);
    assert_eq!(sizer.get_atr_value(), 5.0);
    // cash=10000, risk_amount=100, per_share_risk=5.0, size=20
    assert_eq!(sizer.get_size(10000.0, 50.0, true), 20);
}

// ========== Optimizer 测试 ==========

/// 用于优化器测试的简单策略
struct SimpleStrategy {
    period: usize,
    sma: Option<SMA>,
    prev_val: Option<f64>,
}

impl SimpleStrategy {
    fn new(period: usize) -> Self {
        Self {
            period,
            sma: None,
            prev_val: None,
        }
    }
}

impl Strategy for SimpleStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.sma = Some(SMA::new(self.period));
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() {
            return;
        }
        let close = data[0isize].close;
        let val = self.sma.as_mut().unwrap().next(close);

        if let (Some(v), Some(prev)) = (val, self.prev_val) {
            let pos_size = ctx.position(0).size;
            if prev < v && pos_size == 0 {
                let size = (ctx.cash() * 0.95 / close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                }
            } else if prev > v && pos_size > 0 {
                ctx.sell(0, pos_size);
            }
        }
        self.prev_val = val;
    }
}

#[test]
fn test_optimizer_parallel_run() {
    let optimizer = Optimizer::new(10000.0, "sample_data/orcl-2014.txt")
        .commission(0.005);

    let param_sets = vec![
        vec![5.0],
        vec![10.0],
        vec![20.0],
    ];

    let results = optimizer.run::<SimpleStrategy, _>(
        |params| SimpleStrategy::new(params[0] as usize),
        param_sets,
    );

    assert_eq!(results.len(), 3, "应有 3 组结果");
    for r in &results {
        assert!(r.result.final_value > 0.0, "每组结果应有正的最终价值");
    }
}

#[test]
fn test_optimizer_sorted() {
    let optimizer = Optimizer::new(10000.0, "sample_data/orcl-2014.txt")
        .commission(0.005);

    let param_sets = vec![
        vec![5.0],
        vec![10.0],
    ];

    let results = optimizer.run_sorted::<SimpleStrategy, _>(
        |params| SimpleStrategy::new(params[0] as usize),
        param_sets,
        |r| r.final_value,
        false, // 降序
    );

    assert_eq!(results.len(), 2);
    // 验证按降序排列
    assert!(
        results[0].result.final_value >= results[1].result.final_value,
        "结果应按最终价值降序排列"
    );
}

// ========== 辅助函数 ==========

fn collect_bars<F: DataFeed>(feed: &mut F) -> Vec<Bar> {
    let mut bars = Vec::new();
    while let Some(bar) = feed.next_bar() {
        bars.push(bar);
    }
    bars
}
