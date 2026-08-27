/// 分析器和观察者测试：验证 Phase 4 所有组件的正确性

use chrono::NaiveDate;

use backtrader_rust::analyzers::{
    AnalysisResult, Analyzer, DrawDown, SharpeRatio, SQN, TimeReturn, TradeAnalyzer,
};
use backtrader_rust::brokers::Trade;
use backtrader_rust::core::Bar;
use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::CsvFeed;
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::observers::{BrokerValue, Observer};
use backtrader_rust::strategy::{Context, Strategy};

/// 辅助函数：创建测试用 Bar
fn make_bar(date: (i32, u32, u32), close: f64) -> Bar {
    let dt = NaiveDate::from_ymd_opt(date.0, date.1, date.2)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    Bar::new(dt, close, close, close, close, 1000.0, 0.0)
}

/// 辅助函数：创建已关闭的 Trade
fn make_closed_trade(
    id: u64,
    entry_price: f64,
    exit_price: f64,
    size: i64,
    commission: f64,
) -> Trade {
    let entry_dt = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let exit_dt = NaiveDate::from_ymd_opt(2024, 1, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut trade = Trade::new(id, entry_dt, entry_price, size);
    trade.close(exit_dt, exit_price, commission);
    trade
}

// ========== TimeReturn 测试 ==========

#[test]
fn test_time_return_basic() {
    let mut tr = TimeReturn::new();

    // 组合价值序列：100, 110, 105, 120
    // 收益率：N/A, 0.10, -0.04545, 0.14286
    let values = [100.0, 110.0, 105.0, 120.0];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        tr.next_bar(&bar, v, v);
    }

    let result = tr.stop();
    assert_eq!(result.name, "TimeReturn");

    // 总收益率：(120 - 100) / 100 = 0.20
    let total = result.get("total_return").unwrap();
    assert!((total - 0.20).abs() < 1e-6, "total_return 应为 0.20，实际: {}", total);

    // 周期数：3（4 根 bar 产生 3 个收益率）
    let count = result.get("returns_count").unwrap();
    assert!((count - 3.0).abs() < 1e-6);

    // 最大单周期收益：(120 - 105) / 105 = 0.14286
    let max_ret = result.get("max_return").unwrap();
    let expected_max = (120.0 - 105.0) / 105.0;
    assert!((max_ret - expected_max).abs() < 1e-6, "max_return 应为 {:.6}，实际: {}", expected_max, max_ret);

    // 最小单周期收益（最大亏损）：(105 - 110) / 110 = -0.04545
    let min_ret = result.get("min_return").unwrap();
    let expected_min = (105.0 - 110.0) / 110.0;
    assert!((min_ret - expected_min).abs() < 1e-6, "min_return 应为 {:.6}，实际: {}", expected_min, min_ret);

    // 平均收益率
    let avg_ret = result.get("avg_return").unwrap();
    let expected_avg = (0.10 + (-0.045454545) + 0.142857143) / 3.0;
    assert!((avg_ret - expected_avg).abs() < 1e-4, "avg_return 应为 {:.6}，实际: {}", expected_avg, avg_ret);
}

#[test]
fn test_time_return_empty() {
    let mut tr = TimeReturn::new();
    let result = tr.stop();
    assert_eq!(result.get("returns_count").unwrap(), 0.0);
    assert_eq!(result.get("total_return").unwrap(), 0.0);
}

#[test]
fn test_time_return_single_bar() {
    let mut tr = TimeReturn::new();
    let bar = make_bar((2024, 1, 1), 100.0);
    tr.next_bar(&bar, 100.0, 100.0);
    let result = tr.stop();
    // 单根 bar 没有收益率变化
    assert_eq!(result.get("returns_count").unwrap(), 0.0);
    assert_eq!(result.get("total_return").unwrap(), 0.0);
}

// ========== SharpeRatio 测试 ==========

#[test]
fn test_sharpe_ratio_basic() {
    let mut sr = SharpeRatio::new();

    // 组合价值序列：100, 102, 104, 103, 106, 108
    // 收益率：0.02, 0.01961, -0.00962, 0.02913, 0.01887
    let values = [100.0, 102.0, 104.0, 103.0, 106.0, 108.0];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        sr.next_bar(&bar, v, v);
    }

    let result = sr.stop();
    assert_eq!(result.name, "SharpeRatio");

    let mean = result.get("mean_return").unwrap();
    let std_dev = result.get("std_dev").unwrap();
    let sharpe = result.get("sharpe_ratio").unwrap();

    // 验证均值和标准差为正数
    assert!(mean > 0.0, "平均收益应为正，实际: {}", mean);
    assert!(std_dev > 0.0, "标准差应为正，实际: {}", std_dev);
    assert!(sharpe > 0.0, "夏普比率应为正（均值为正），实际: {}", sharpe);

    // 手动验证公式：sharpe = mean / std_dev * sqrt(252)
    let expected_sharpe = mean / std_dev * (252.0_f64).sqrt();
    assert!(
        (sharpe - expected_sharpe).abs() < 1e-6,
        "sharpe 应为 {:.6}，实际: {:.6}",
        expected_sharpe,
        sharpe
    );
}

#[test]
fn test_sharpe_ratio_with_risk_free_rate() {
    let mut sr = SharpeRatio::with_params(0.05, 252.0);

    // 使用恒定收益率序列
    let values = [100.0, 100.1, 100.2, 100.3, 100.4];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        sr.next_bar(&bar, v, v);
    }

    let result = sr.stop();
    let sharpe = result.get("sharpe_ratio").unwrap();
    let mean = result.get("mean_return").unwrap();

    // 每期无风险利率 = 0.05 / 252 ≈ 0.000198
    let rf_per_period = 0.05 / 252.0;
    // 平均收益率 ≈ 0.001
    // 由于收益恒定，std_dev ≈ 0，所以 sharpe 应该很大或接近无穷
    assert!(mean > rf_per_period, "均值应大于每期无风险利率");
    assert!(sharpe > 0.0);
}

#[test]
fn test_sharpe_ratio_empty() {
    let mut sr = SharpeRatio::new();
    let result = sr.stop();
    assert_eq!(result.get("sharpe_ratio").unwrap(), 0.0);
    assert_eq!(result.get("mean_return").unwrap(), 0.0);
    assert_eq!(result.get("std_dev").unwrap(), 0.0);
}

// ========== DrawDown 测试 ==========

#[test]
fn test_drawdown_basic() {
    let mut dd = DrawDown::new();

    // 组合价值序列：100, 110, 105, 95, 108, 115, 100, 120
    // 峰值：         100, 110, 110, 110, 110, 115, 115, 120
    // 回撤%：        0,   0,   4.55, 13.64, 1.82, 0,  13.04, 0
    // 最大回撤：13.64%（110 -> 95）
    let values = [100.0, 110.0, 105.0, 95.0, 108.0, 115.0, 100.0, 120.0];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        dd.next_bar(&bar, v, v);
    }

    let result = dd.stop();
    assert_eq!(result.name, "DrawDown");

    // 最大回撤：(110 - 95) / 110 * 100 = 13.636%
    let max_dd = result.get("max_drawdown").unwrap();
    let expected_max_dd = (110.0 - 95.0) / 110.0 * 100.0;
    assert!(
        (max_dd - expected_max_dd).abs() < 1e-3,
        "max_drawdown 应为 {:.3}%，实际: {:.3}%",
        expected_max_dd,
        max_dd
    );

    // 最大回撤金额：110 - 95 = 15
    let max_dd_value = result.get("max_drawdown_value").unwrap();
    assert!(
        (max_dd_value - 15.0).abs() < 1e-6,
        "max_drawdown_value 应为 15.0，实际: {}",
        max_dd_value
    );

    // 最大回撤时的峰值：110
    let max_dd_peak = result.get("max_drawdown_peak").unwrap();
    assert!(
        (max_dd_peak - 110.0).abs() < 1e-6,
        "max_drawdown_peak 应为 110.0，实际: {}",
        max_dd_peak
    );

    // 最终价值 120 > 峰值 115，所以当前回撤为 0
    let current_dd = result.get("current_drawdown").unwrap();
    assert!((current_dd - 0.0).abs() < 1e-6, "current_drawdown 应为 0，实际: {}", current_dd);
}

#[test]
fn test_drawdown_longest_bars() {
    let mut dd = DrawDown::new();

    // 序列：100, 90, 85, 80, 110
    // 从 bar 1（90）开始回撤，持续到 bar 3（80），共 3 根 bar
    // bar 4（110）创新高，回撤重置
    let values = [100.0, 90.0, 85.0, 80.0, 110.0];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        dd.next_bar(&bar, v, v);
    }

    let result = dd.stop();
    let longest = result.get("longest_drawdown_bars").unwrap();
    assert!(
        (longest - 3.0).abs() < 1e-6,
        "longest_drawdown_bars 应为 3，实际: {}",
        longest
    );
}

#[test]
fn test_drawdown_no_drawdown() {
    let mut dd = DrawDown::new();

    // 单调递增序列：无回撤
    let values = [100.0, 101.0, 102.0, 103.0];
    for (i, &v) in values.iter().enumerate() {
        let bar = make_bar((2024, 1, (i + 1) as u32), v);
        dd.next_bar(&bar, v, v);
    }

    let result = dd.stop();
    assert_eq!(result.get("max_drawdown").unwrap(), 0.0);
    assert_eq!(result.get("longest_drawdown_bars").unwrap(), 0.0);
}

// ========== TradeAnalyzer 测试 ==========

#[test]
fn test_trade_analyzer_basic() {
    let mut ta = TradeAnalyzer::new();

    // 4 笔交易：
    // #1: 盈利 100 (入场 100, 出场 110, 10股, 手续费 0)
    // #2: 亏损 -50 (入场 100, 出场 95, 10股, 手续费 0)
    // #3: 盈利 200 (入场 100, 出场 120, 10股, 手续费 0)
    // #4: 亏损 -30 (入场 100, 出场 97, 10股, 手续费 0)
    ta.on_trade(&make_closed_trade(1, 100.0, 110.0, 10, 0.0)); // pnl = 100
    ta.on_trade(&make_closed_trade(2, 100.0, 95.0, 10, 0.0));  // pnl = -50
    ta.on_trade(&make_closed_trade(3, 100.0, 120.0, 10, 0.0)); // pnl = 200
    ta.on_trade(&make_closed_trade(4, 100.0, 97.0, 10, 0.0));  // pnl = -30

    let result = ta.stop();
    assert_eq!(result.name, "TradeAnalyzer");

    // 总交易数
    assert_eq!(result.get("total_trades").unwrap(), 4.0);
    // 盈利交易数
    assert_eq!(result.get("won").unwrap(), 2.0);
    // 亏损交易数
    assert_eq!(result.get("lost").unwrap(), 2.0);
    // 胜率
    assert_eq!(result.get("win_rate").unwrap(), 50.0);
    // 平均盈利 = (100 + 200) / 2 = 150
    assert!((result.get("avg_win").unwrap() - 150.0).abs() < 1e-6);
    // 平均亏损 = (-50 + -30) / 2 = -40
    assert!((result.get("avg_loss").unwrap() - (-40.0)).abs() < 1e-6);
    // 最大单笔盈利 = 200
    assert!((result.get("max_win").unwrap() - 200.0).abs() < 1e-6);
    // 最大单笔亏损 = -50
    assert!((result.get("max_loss").unwrap() - (-50.0)).abs() < 1e-6);
    // 盈利因子 = (100 + 200) / (50 + 30) = 300/80 = 3.75
    assert!((result.get("profit_factor").unwrap() - 3.75).abs() < 1e-6);
    // 总盈亏 = 100 - 50 + 200 - 30 = 220
    assert!((result.get("total_pnl").unwrap() - 220.0).abs() < 1e-6);
    // 平均盈亏 = 220 / 4 = 55
    assert!((result.get("avg_pnl").unwrap() - 55.0).abs() < 1e-6);
}

#[test]
fn test_trade_analyzer_all_winners() {
    let mut ta = TradeAnalyzer::new();

    ta.on_trade(&make_closed_trade(1, 100.0, 110.0, 10, 0.0));
    ta.on_trade(&make_closed_trade(2, 100.0, 120.0, 10, 0.0));

    let result = ta.stop();
    assert_eq!(result.get("total_trades").unwrap(), 2.0);
    assert_eq!(result.get("won").unwrap(), 2.0);
    assert_eq!(result.get("lost").unwrap(), 0.0);
    assert_eq!(result.get("win_rate").unwrap(), 100.0);
    // 盈利因子：无亏损，应为无穷大
    assert!(result.get("profit_factor").unwrap().is_infinite());
}

#[test]
fn test_trade_analyzer_empty() {
    let mut ta = TradeAnalyzer::new();
    let result = ta.stop();
    assert_eq!(result.get("total_trades").unwrap(), 0.0);
    assert_eq!(result.get("win_rate").unwrap(), 0.0);
}

#[test]
fn test_trade_analyzer_with_commission() {
    let mut ta = TradeAnalyzer::new();

    // 入场 100, 出场 105, 10 股, 手续费 20
    // pnl = (105 - 100) * 10 - 20 = 50 - 20 = 30
    ta.on_trade(&make_closed_trade(1, 100.0, 105.0, 10, 20.0));

    let result = ta.stop();
    assert!((result.get("total_pnl").unwrap() - 30.0).abs() < 1e-6);
}

// ========== SQN 测试 ==========

#[test]
fn test_sqn_basic() {
    let mut sqn = SQN::new();

    // 4 笔交易 pnl：100, 200, -50, 150
    sqn.on_trade(&make_closed_trade(1, 100.0, 110.0, 10, 0.0));  // pnl = 100
    sqn.on_trade(&make_closed_trade(2, 100.0, 120.0, 10, 0.0));  // pnl = 200
    sqn.on_trade(&make_closed_trade(3, 100.0, 95.0, 10, 0.0));   // pnl = -50
    sqn.on_trade(&make_closed_trade(4, 100.0, 115.0, 10, 0.0));  // pnl = 150

    let result = sqn.stop();
    assert_eq!(result.name, "SQN");
    assert_eq!(result.get("trades").unwrap(), 4.0);

    // 手动计算：
    // mean = (100 + 200 - 50 + 150) / 4 = 400/4 = 100
    // variance = ((100-100)^2 + (200-100)^2 + (-50-100)^2 + (150-100)^2) / 4
    //          = (0 + 10000 + 22500 + 2500) / 4 = 35000/4 = 8750
    // std_dev = sqrt(8750) = 93.5414...
    // SQN = sqrt(4) * 100 / 93.5414 = 2 * 100 / 93.5414 = 2.1381...
    let mean: f64 = 100.0;
    let variance: f64 = 8750.0;
    let std_dev = variance.sqrt();
    let expected_sqn = 4.0_f64.sqrt() * mean / std_dev;

    let sqn_val = result.get("sqn").unwrap();
    assert!(
        (sqn_val - expected_sqn).abs() < 1e-3,
        "SQN 应为 {:.4}，实际: {:.4}",
        expected_sqn,
        sqn_val
    );

    // 期望值 = 100
    let expectancy = result.get("expectancy").unwrap();
    assert!((expectancy - 100.0).abs() < 1e-6);
}

#[test]
fn test_sqn_empty() {
    let mut sqn = SQN::new();
    let result = sqn.stop();
    assert_eq!(result.get("trades").unwrap(), 0.0);
    assert_eq!(result.get("sqn").unwrap(), 0.0);
    assert_eq!(result.get("expectancy").unwrap(), 0.0);
}

#[test]
fn test_sqn_single_trade() {
    let mut sqn = SQN::new();
    sqn.on_trade(&make_closed_trade(1, 100.0, 110.0, 10, 0.0));

    let result = sqn.stop();
    // 单笔交易：std_dev = 0，mean = 100 > 0，SQN = infinity
    let sqn_val = result.get("sqn").unwrap();
    assert!(sqn_val.is_infinite() && sqn_val > 0.0);
}

// ========== BrokerValue Observer 测试 ==========

#[test]
fn test_broker_value_observer() {
    let mut bv = BrokerValue::new();

    // 模拟 3 根 bar 的记录
    let bar1 = make_bar((2024, 1, 1), 100.0);
    let bar2 = make_bar((2024, 1, 2), 102.0);
    let bar3 = make_bar((2024, 1, 3), 99.0);

    bv.next(0, &bar1, 10000.0, 9000.0);
    bv.next(1, &bar2, 10200.0, 9000.0);
    bv.next(2, &bar3, 9900.0, 9100.0);

    // 验证记录数
    assert_eq!(bv.len(), 3);
    assert!(!bv.is_empty());

    // 验证最终价值
    assert!((bv.final_value().unwrap() - 9900.0).abs() < 1e-6);
    assert!((bv.final_cash().unwrap() - 9100.0).abs() < 1e-6);

    // 验证每条记录
    let vals = bv.values();
    assert_eq!(vals[0], (0, 10000.0, 9000.0));
    assert_eq!(vals[1], (1, 10200.0, 9000.0));
    assert_eq!(vals[2], (2, 9900.0, 9100.0));
}

#[test]
fn test_broker_value_observer_empty() {
    let bv = BrokerValue::new();
    assert!(bv.is_empty());
    assert!(bv.final_value().is_none());
    assert!(bv.final_cash().is_none());
}

// ========== 分析结果 AnalysisResult 测试 ==========

#[test]
fn test_analysis_result_set_get() {
    let mut ar = AnalysisResult::new("TestAnalyzer");
    assert_eq!(ar.name, "TestAnalyzer");

    ar.set("metric_a", 1.5);
    ar.set("metric_b", -0.003);

    assert!((ar.get("metric_a").unwrap() - 1.5).abs() < 1e-9);
    assert!((ar.get("metric_b").unwrap() - (-0.003)).abs() < 1e-9);
    assert!(ar.get("metric_c").is_none());
}

// ========== 集成测试：完整回测 + 分析器 + 观察者 ==========

/// 简单 SMA 交叉策略（用于集成测试）
struct SimpleSmaStrategy {
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl Strategy for SimpleSmaStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        self.fast_sma = Some(SMA::new(5));
        self.slow_sma = Some(SMA::new(15));
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

        let pos = ctx.position(0);

        if let (Some(pf), Some(ps)) = (self.prev_fast, self.prev_slow) {
            if pf <= ps && fast > slow && !pos.is_open() {
                let size = ((ctx.cash() * 0.90) / close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                }
            } else if pf >= ps && fast < slow && pos.is_open() {
                ctx.sell(0, pos.size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }
}

#[test]
fn test_full_backtest_with_analyzers_and_observers() {
    let feed = CsvFeed::new("sample_data/orcl-2014.txt").expect("应能加载数据");

    let strategy = SimpleSmaStrategy {
        fast_sma: None,
        slow_sma: None,
        prev_fast: None,
        prev_slow: None,
    };

    let result = CerebroBuilder::new()
        .cash(10000.0)
        .commission(0.005)
        .add_data(Box::new(feed))
        .add_strategy(Box::new(strategy))
        // 添加所有分析器
        .add_analyzer(Box::new(TimeReturn::new()))
        .add_analyzer(Box::new(SharpeRatio::new()))
        .add_analyzer(Box::new(DrawDown::new()))
        .add_analyzer(Box::new(TradeAnalyzer::new()))
        .add_analyzer(Box::new(SQN::new()))
        // 添加观察者
        .add_observer(Box::new(BrokerValue::new()))
        .run();

    // 验证回测基本运行
    assert_eq!(result.bars_processed, 252, "应处理 252 根 bar");
    assert!(result.final_value > 0.0, "最终价值应为正数");

    // 验证有 5 个分析器结果
    assert_eq!(
        result.analyzer_results.len(),
        5,
        "应有 5 个分析器结果，实际: {}",
        result.analyzer_results.len()
    );

    // 验证 TimeReturn 分析器
    let time_return = result
        .analyzer_results
        .iter()
        .find(|r| r.name == "TimeReturn")
        .expect("应有 TimeReturn 分析器结果");
    let returns_count = time_return.get("returns_count").unwrap();
    // 252 根 bar 产生 251 个收益率
    assert!(
        (returns_count - 251.0).abs() < 1e-6,
        "returns_count 应为 251，实际: {}",
        returns_count
    );

    // 验证 SharpeRatio 分析器
    let sharpe = result
        .analyzer_results
        .iter()
        .find(|r| r.name == "SharpeRatio")
        .expect("应有 SharpeRatio 分析器结果");
    assert!(sharpe.get("sharpe_ratio").is_some());
    assert!(sharpe.get("mean_return").is_some());
    assert!(sharpe.get("std_dev").is_some());

    // 验证 DrawDown 分析器
    let dd = result
        .analyzer_results
        .iter()
        .find(|r| r.name == "DrawDown")
        .expect("应有 DrawDown 分析器结果");
    let max_dd = dd.get("max_drawdown").unwrap();
    assert!(max_dd >= 0.0, "最大回撤应非负，实际: {}", max_dd);

    // 验证 TradeAnalyzer
    let ta = result
        .analyzer_results
        .iter()
        .find(|r| r.name == "TradeAnalyzer")
        .expect("应有 TradeAnalyzer 分析器结果");
    let total_trades = ta.get("total_trades").unwrap();
    assert!(
        total_trades >= 0.0,
        "交易数应非负，实际: {}",
        total_trades
    );
    // 如果有交易，胜率应在 0-100 之间
    if total_trades > 0.0 {
        let win_rate = ta.get("win_rate").unwrap();
        assert!(
            (0.0..=100.0).contains(&win_rate),
            "胜率应在 0-100 之间，实际: {}",
            win_rate
        );
    }

    // 验证 SQN 分析器
    let sqn = result
        .analyzer_results
        .iter()
        .find(|r| r.name == "SQN")
        .expect("应有 SQN 分析器结果");
    assert!(sqn.get("sqn").is_some());
    assert!(sqn.get("expectancy").is_some());

    // 调用 print_summary 验证不 panic
    result.print_summary();
}

#[test]
fn test_backtest_without_analyzers_backward_compat() {
    // 验证不添加分析器时仍然正常运行（向后兼容）
    let feed = CsvFeed::new("sample_data/orcl-2014.txt").expect("应能加载数据");

    let strategy = SimpleSmaStrategy {
        fast_sma: None,
        slow_sma: None,
        prev_fast: None,
        prev_slow: None,
    };

    let result = CerebroBuilder::new()
        .cash(10000.0)
        .commission(0.005)
        .add_data(Box::new(feed))
        .add_strategy(Box::new(strategy))
        .run();

    assert_eq!(result.bars_processed, 252);
    assert!(result.final_value > 0.0);
    assert!(result.analyzer_results.is_empty(), "无分析器时结果应为空");
}
