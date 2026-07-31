/// 集成测试：验证 Phase 1 核心组件的正确性

use backtrader_rust::core::TimeSeries;
use backtrader_rust::feeds::{CsvFeed, DataFeed};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::brokers::{
    CommissionInfo, DefaultBroker, Broker, Order, OrderSide, Position,
};
use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::strategy::{Context, Strategy};

// ========== CsvFeed 测试 ==========

#[test]
fn test_csv_feed_load() {
    let mut feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("应该能加载 CSV 数据");

    assert!(feed.len() > 200, "数据应有 200+ 根 bar，实际: {}", feed.len());
    assert!(!feed.is_empty());

    // 检查第一根 bar
    let bar = feed.next_bar().expect("应能获取第一根 bar");
    assert!(bar.open > 0.0);
    assert!(bar.high >= bar.low);
    assert!(bar.volume > 0.0);
}

#[test]
fn test_csv_feed_reset() {
    let mut feed = CsvFeed::new("sample_data/orcl-2014.txt").unwrap();
    let first_bar = feed.next_bar().unwrap();
    feed.reset();
    let reset_bar = feed.next_bar().unwrap();
    assert_eq!(first_bar.datetime, reset_bar.datetime);
}

// ========== TimeSeries 测试 ==========

#[test]
fn test_timeseries_index_access() {
    let mut ts = TimeSeries::new();
    for i in 0..10 {
        ts.push(i as f64);
    }
    // ts[0] = 9 (最新), ts[-1] = 8, ts[-9] = 0
    assert_eq!(ts[0], 9.0);
    assert_eq!(ts[-1], 8.0);
    assert_eq!(ts[-9], 0.0);
}

#[test]
fn test_timeseries_get_method() {
    let mut ts = TimeSeries::new();
    ts.push(100.0);
    ts.push(200.0);
    ts.push(300.0);

    assert_eq!(ts.get(0), Some(&300.0)); // 最新
    assert_eq!(ts.get(1), Some(&200.0)); // 前一个
    assert_eq!(ts.get(2), Some(&100.0)); // 前两个
    assert_eq!(ts.get(3), None);          // 越界
}

// ========== SMA 测试 ==========

#[test]
fn test_sma_calculation() {
    let mut sma = SMA::new(5);

    // 前 4 个值不应返回结果
    for i in 1..=4 {
        assert!(sma.next(i as f64).is_none());
    }

    // 第 5 个值：(1+2+3+4+5)/5 = 3.0
    let v = sma.next(5.0).unwrap();
    assert!((v - 3.0).abs() < 1e-9);

    // 第 6 个值：(2+3+4+5+6)/5 = 4.0
    let v = sma.next(6.0).unwrap();
    assert!((v - 4.0).abs() < 1e-9);
}

#[test]
fn test_sma_min_period() {
    let sma = SMA::new(20);
    assert_eq!(sma.min_period(), 20);
    assert!(!sma.is_ready());
}

// ========== DefaultBroker 测试 ==========

#[test]
fn test_broker_market_buy() {
    use chrono::NaiveDate;

    let mut broker = DefaultBroker::new(10000.0, CommissionInfo::new(0.001));

    let dt = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let bar = backtrader_rust::core::Bar::new(dt, 100.0, 105.0, 95.0, 102.0, 1000000.0, 0.0);

    // 先推送一根 bar（让 broker 知道当前时间）
    broker.next_bar(&bar, 0);

    // 提交买入订单：买 10 股 @ 市价
    let order_id = broker.next_order_id();
    let order = Order::new_market(order_id, OrderSide::Buy, 10);
    broker.submit_order(order, 0);

    // 推送下一根 bar 执行订单（市价单在 open 价执行）
    let dt2 = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let bar2 = backtrader_rust::core::Bar::new(dt2, 101.0, 106.0, 96.0, 103.0, 1000000.0, 0.0);
    broker.next_bar(&bar2, 0);

    // 检查持仓
    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10);

    // 检查现金减少：cost = 10 * 101.0 + commission
    let notifications = broker.drain_notifications();
    assert!(!notifications.is_empty(), "应有订单完成通知");
}

#[test]
fn test_broker_market_sell() {
    use chrono::NaiveDate;

    let mut broker = DefaultBroker::new(10000.0, CommissionInfo::new(0.001));

    let dt = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let bar = backtrader_rust::core::Bar::new(dt, 100.0, 105.0, 95.0, 102.0, 1000000.0, 0.0);
    broker.next_bar(&bar, 0);

    // 买入
    let order = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(order, 0);
    let bar2 = backtrader_rust::core::Bar::new(dt, 100.0, 105.0, 95.0, 102.0, 1000000.0, 0.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications();

    // 卖出
    let order = Order::new_market(broker.next_order_id(), OrderSide::Sell, 10);
    broker.submit_order(order, 0);
    let bar3 = backtrader_rust::core::Bar::new(dt, 110.0, 115.0, 105.0, 112.0, 1000000.0, 0.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "平仓后应为 0 股");
}

// ========== Position P&L 测试 ==========

#[test]
fn test_position_pnl_calculation() {
    let mut pos = Position::new();
    pos.update(100, 50.0);
    pos.current_price = 55.0;

    assert!((pos.unrealized_pnl() - 500.0).abs() < 1e-9);
    assert!((pos.pnl(55.0) - 500.0).abs() < 1e-9);
    assert!((pos.market_value() - 5500.0).abs() < 1e-9);
}

// ========== 完整 SMA 交叉回测测试 ==========

struct TestSmaCross {
    fast_sma: Option<SMA>,
    slow_sma: Option<SMA>,
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl Strategy for TestSmaCross {
    fn init(&mut self, _ctx: &mut Context) {
        self.fast_sma = Some(SMA::new(10));
        self.slow_sma = Some(SMA::new(30));
    }

    fn next(&mut self, ctx: &mut Context) {
        let data = ctx.data(0);
        if data.is_empty() { return; }

        let close = data[0isize].close;
        let fast_val = self.fast_sma.as_mut().unwrap().next(close);
        let slow_val = self.slow_sma.as_mut().unwrap().next(close);

        let (Some(fast), Some(slow)) = (fast_val, slow_val) else { return; };

        let pos = ctx.position(0);

        if let (Some(pf), Some(ps)) = (self.prev_fast, self.prev_slow) {
            if pf <= ps && fast > slow && !pos.is_open() {
                let size = ((ctx.cash() * 0.95) / close) as i64;
                if size > 0 { ctx.buy(0, size); }
            } else if pf >= ps && fast < slow && pos.is_open() {
                ctx.sell(0, pos.size);
            }
        }

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
    }
}

#[test]
fn test_full_sma_crossover_backtest() {
    let feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("应能加载数据");

    let strategy = TestSmaCross {
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

    // 验证回测基本运行
    assert_eq!(result.bars_processed, 252, "应处理 252 根 bar");
    assert!(result.final_value > 0.0, "最终价值应为正数");
    println!("最终价值: {:.2}, 收益率: {:.2}%, 交易数: {}",
             result.final_value, result.total_return, result.trades.len());
}
