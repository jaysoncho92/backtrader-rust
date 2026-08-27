/// Broker 全面单元测试
/// 覆盖 Limit、Stop、StopLimit、OCO、Bracket、CommissionType、Slippage 等

use chrono::NaiveDate;
use backtrader_rust::brokers::{
    Broker, CommissionInfo, CommissionType, DefaultBroker, Order, OrderNotification,
    OrderSide, Slippage,
};
use backtrader_rust::core::Bar;

/// 辅助函数：创建指定日期的 Bar
fn make_bar(date: (i32, u32, u32), open: f64, high: f64, low: f64, close: f64) -> Bar {
    let dt = NaiveDate::from_ymd_opt(date.0, date.1, date.2)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    Bar::new(dt, open, high, low, close, 1_000_000.0, 0.0)
}

/// 辅助函数：创建 Broker 并推送初始 bar
fn setup_broker(cash: f64) -> DefaultBroker {
    DefaultBroker::new(cash, CommissionInfo::new(0.001))
}

// ========== Limit Buy 单测试 ==========

#[test]
fn test_limit_buy_in_range() {
    // Buy Limit @ 95.0，bar.low=93.0 <= 95.0 -> 应以 95.0 成交
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 98.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_limit(broker.next_order_id(), OrderSide::Buy, 10, 95.0);
    broker.submit_order(order, 0);

    // 下一根 bar：low=93.0 <= 95.0，应执行
    let bar2 = make_bar((2024, 1, 2), 96.0, 100.0, 93.0, 99.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "应以限价 95.0 买入 10 股");
    assert!((pos.price - 95.0).abs() < 1e-9, "成交价应为 limit_price=95.0");
}

#[test]
fn test_limit_buy_out_of_range() {
    // Buy Limit @ 90.0，bar.low=93.0 > 90.0 -> 不应成交
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 98.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_limit(broker.next_order_id(), OrderSide::Buy, 10, 90.0);
    broker.submit_order(order, 0);

    let bar2 = make_bar((2024, 1, 2), 96.0, 100.0, 93.0, 99.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "限价未到，不应成交");
}

// ========== Limit Sell 单测试 ==========

#[test]
fn test_limit_sell_in_range() {
    // 先买入，再挂 Sell Limit @ 110.0，bar.high=112.0 >= 110.0 -> 应以 110.0 成交
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    // 买入
    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(buy, 0);
    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications();

    // 挂 Sell Limit
    let sell = Order::new_limit(broker.next_order_id(), OrderSide::Sell, 10, 110.0);
    broker.submit_order(sell, 0);

    let bar3 = make_bar((2024, 1, 3), 108.0, 112.0, 105.0, 111.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Sell Limit 应以 110.0 成交并平仓");
}

#[test]
fn test_limit_sell_out_of_range() {
    // Sell Limit @ 115.0，bar.high=112.0 < 115.0 -> 不应成交
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(buy, 0);
    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications();

    let sell = Order::new_limit(broker.next_order_id(), OrderSide::Sell, 10, 115.0);
    broker.submit_order(sell, 0);

    let bar3 = make_bar((2024, 1, 3), 108.0, 112.0, 105.0, 111.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "Sell Limit 未到价，应仍持仓");
}

// ========== Stop Buy 单测试 ==========

#[test]
fn test_stop_buy_trigger() {
    // Buy Stop @ 110.0，bar.high=112.0 >= 110.0 -> 触发，以 open=111.0 执行
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_stop(broker.next_order_id(), OrderSide::Buy, 10, 110.0);
    broker.submit_order(order, 0);

    let bar2 = make_bar((2024, 1, 2), 111.0, 112.0, 108.0, 110.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "Stop Buy 应触发并买入");
    // 以 open=111.0 执行
    assert!((pos.price - 111.0).abs() < 1e-9, "应以 open 价 111.0 成交");
}

#[test]
fn test_stop_buy_no_trigger() {
    // Buy Stop @ 115.0，bar.high=112.0 < 115.0 -> 不触发
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_stop(broker.next_order_id(), OrderSide::Buy, 10, 115.0);
    broker.submit_order(order, 0);

    let bar2 = make_bar((2024, 1, 2), 105.0, 112.0, 100.0, 110.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Stop 未触发，不应成交");
}

// ========== Stop Sell 单测试 ==========

#[test]
fn test_stop_sell_trigger() {
    // 先买入，再 Sell Stop @ 95.0，bar.low=93.0 <= 95.0 -> 触发
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(buy, 0);
    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications();

    let sell = Order::new_stop(broker.next_order_id(), OrderSide::Sell, 10, 95.0);
    broker.submit_order(sell, 0);

    let bar3 = make_bar((2024, 1, 3), 96.0, 97.0, 93.0, 94.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Stop Sell 应触发并卖出平仓");
}

#[test]
fn test_stop_sell_no_trigger() {
    // Sell Stop @ 90.0，bar.low=93.0 > 90.0 -> 不触发
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(buy, 0);
    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications();

    let sell = Order::new_stop(broker.next_order_id(), OrderSide::Sell, 10, 90.0);
    broker.submit_order(sell, 0);

    let bar3 = make_bar((2024, 1, 3), 96.0, 100.0, 93.0, 98.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "Stop Sell 未触发，应仍持仓");
}

// ========== StopLimit 单测试 ==========

#[test]
fn test_stop_limit_buy_triggered_and_filled() {
    // StopLimit Buy: stop=110, limit=112
    // bar2: high=113 >= 110 -> 触发; low=108 <= 112 -> 以 112 执行
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_stop_limit(
        broker.next_order_id(),
        OrderSide::Buy,
        10,
        110.0,
        112.0,
    );
    broker.submit_order(order, 0);

    let bar2 = make_bar((2024, 1, 2), 109.0, 113.0, 108.0, 111.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "StopLimit Buy 应触发并以限价成交");
    assert!((pos.price - 112.0).abs() < 1e-9, "应以 limit_price=112.0 成交");
}

#[test]
fn test_stop_limit_buy_triggered_but_not_filled() {
    // StopLimit Buy: stop=110, limit=108
    // bar2: high=113 >= 110 -> 触发; low=109 > 108 -> 限价未满足，不执行
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let order = Order::new_stop_limit(
        broker.next_order_id(),
        OrderSide::Buy,
        10,
        110.0,
        108.0,
    );
    broker.submit_order(order, 0);

    let bar2 = make_bar((2024, 1, 2), 109.0, 113.0, 109.0, 111.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "StopLimit 触发但限价未满足，不应成交");
}

// ========== OCO 逻辑测试 ==========

#[test]
fn test_oco_one_fills_cancels_other() {
    // 两个 Buy Limit 在同一 OCO 组：一个成交，另一个应被取消
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let oco_id = 100u64;
    let mut order_a = Order::new_limit(broker.next_order_id(), OrderSide::Buy, 10, 95.0);
    order_a.oco_group = Some(oco_id);
    let mut order_b = Order::new_limit(broker.next_order_id(), OrderSide::Buy, 10, 90.0);
    order_b.oco_group = Some(oco_id);

    broker.submit_order(order_a, 0);
    broker.submit_order(order_b, 0);

    // bar2: low=93.0 <= 95.0 -> order_a 成交; low=93.0 > 90.0 -> order_b 不成交
    // OCO: order_a 成交 -> order_b 被取消
    let bar2 = make_bar((2024, 1, 2), 96.0, 100.0, 93.0, 99.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "order_a 应成交买入 10 股");

    let notifications = broker.drain_notifications();
    let canceled = notifications.iter().filter(|n| matches!(n, OrderNotification::OrderCanceled(_))).count();
    assert!(canceled >= 1, "OCO 组中应有至少一个订单被取消");
}

// ========== Bracket 订单测试 ==========

#[test]
fn test_bracket_order_entry_fills_activates_children() {
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    // Bracket: Buy Limit @ 98, Take Profit @ 110, Stop Loss @ 92
    let (_entry_id, _tp_id, _sl_id) =
        broker.bracket_order(OrderSide::Buy, 10, 98.0, 110.0, 92.0, 0);

    // bar2: low=96 <= 98 -> entry 成交
    let bar2 = make_bar((2024, 1, 2), 99.0, 102.0, 96.0, 100.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "Bracket 主单应成交");

    // bar3: high=112 >= 110 -> Take Profit 应触发
    let bar3 = make_bar((2024, 1, 3), 105.0, 112.0, 104.0, 111.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Take Profit 应触发并平仓");
}

#[test]
fn test_bracket_order_stop_loss_triggers() {
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    // Bracket: Buy Limit @ 98, Take Profit @ 115, Stop Loss @ 93
    let _ = broker.bracket_order(OrderSide::Buy, 10, 98.0, 115.0, 93.0, 0);

    // bar2: low=96 <= 98 -> entry 成交
    let bar2 = make_bar((2024, 1, 2), 99.0, 102.0, 96.0, 100.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "主单应成交");

    // bar3: low=91 <= 93 -> Stop Loss 触发
    let bar3 = make_bar((2024, 1, 3), 94.0, 95.0, 91.0, 92.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Stop Loss 应触发并平仓");
}

// ========== CommissionType 测试 ==========

#[test]
fn test_commission_percent() {
    let ci = CommissionInfo::from_type(CommissionType::Percent { rate: 0.01 });
    // 100 股 @ 50.0, 1% -> 100 * 50 * 0.01 = 50.0
    let c = ci.calculate(100, 50.0);
    assert!((c - 50.0).abs() < 1e-9, "Percent 佣金应为 50.0, 实际: {}", c);
}

#[test]
fn test_commission_fixed() {
    let ci = CommissionInfo::from_type(CommissionType::Fixed { amount: 9.99 });
    let c = ci.calculate(100, 50.0);
    assert!((c - 9.99).abs() < 1e-9, "Fixed 佣金应为 9.99, 实际: {}", c);
}

#[test]
fn test_commission_fixed_plus_percent() {
    let ci = CommissionInfo::from_type(CommissionType::FixedPlusPercent {
        fixed: 5.0,
        rate: 0.005,
    });
    // 100 股 @ 50.0 -> 5.0 + 100 * 50 * 0.005 = 5.0 + 25.0 = 30.0
    let c = ci.calculate(100, 50.0);
    assert!((c - 30.0).abs() < 1e-9, "FixedPlusPercent 佣金应为 30.0, 实际: {}", c);
}

#[test]
fn test_commission_per_share() {
    let ci = CommissionInfo::from_type(CommissionType::PerShare {
        amount: 0.01,
        min: 1.0,
    });
    // 100 股 -> 100 * 0.01 = 1.0 (等于 min)
    let c = ci.calculate(100, 50.0);
    assert!((c - 1.0).abs() < 1e-9, "PerShare 佣金应为 1.0, 实际: {}", c);

    // 50 股 -> 50 * 0.01 = 0.5 < min=1.0 -> 使用 min
    let c2 = ci.calculate(50, 50.0);
    assert!((c2 - 1.0).abs() < 1e-9, "PerShare 佣金应为 min=1.0, 实际: {}", c2);
}

#[test]
fn test_commission_backward_compat() {
    // 兼容 Phase 1 的 CommissionInfo::new(rate)
    let ci = CommissionInfo::new(0.005);
    // 100 股 @ 100.0 -> 100 * 100 * 0.005 = 50.0
    let c = ci.calculate(100, 100.0);
    assert!((c - 50.0).abs() < 1e-9, "向后兼容佣金应为 50.0, 实际: {}", c);
}

// ========== Slippage 测试 ==========

#[test]
fn test_slippage_fixed_buy() {
    let slip = Slippage::Fixed(0.5);
    // 买入：价格向上滑 0.5
    let p = slip.apply(100.0, true);
    assert!((p - 100.5).abs() < 1e-9, "Fixed slippage buy 应为 100.5, 实际: {}", p);
}

#[test]
fn test_slippage_fixed_sell() {
    let slip = Slippage::Fixed(0.5);
    // 卖出：价格向下滑 0.5
    let p = slip.apply(100.0, false);
    assert!((p - 99.5).abs() < 1e-9, "Fixed slippage sell 应为 99.5, 实际: {}", p);
}

#[test]
fn test_slippage_percent_buy() {
    let slip = Slippage::Percent(0.01);
    // 买入：价格向上 1%
    let p = slip.apply(100.0, true);
    assert!((p - 101.0).abs() < 1e-9, "Percent slippage buy 应为 101.0, 实际: {}", p);
}

#[test]
fn test_slippage_percent_sell() {
    let slip = Slippage::Percent(0.01);
    // 卖出：价格向下 1%
    let p = slip.apply(100.0, false);
    assert!((p - 99.0).abs() < 1e-9, "Percent slippage sell 应为 99.0, 实际: {}", p);
}

#[test]
fn test_market_order_with_slippage() {
    // 测试市价单含滑点
    let mut broker = DefaultBroker::new(10000.0, CommissionInfo::new(0.001));
    broker.set_slippage(Slippage::Fixed(0.5));

    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10);
    broker.submit_order(buy, 0);

    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 10, "带滑点的市价买入应成交");
    // 执行价 = open + slippage = 100.0 + 0.5 = 100.5
    assert!(
        (pos.price - 100.5).abs() < 1e-9,
        "带滑点成交价应为 100.5, 实际: {}",
        pos.price
    );
}

// ========== 资金/仓位不足测试 ==========

#[test]
fn test_insufficient_cash_rejected() {
    // 资金不足买入应被拒绝
    let mut broker = setup_broker(100.0); // 只有 100 元
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 10); // 10 * 100 = 1000 > 100
    broker.submit_order(buy, 0);

    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "资金不足，订单应被拒绝");

    let notifications = broker.drain_notifications();
    let rejected = notifications.iter().any(|n| matches!(n, OrderNotification::OrderRejected(_)));
    assert!(rejected, "应有订单被拒绝的通知");
}

#[test]
fn test_insufficient_position_rejected() {
    // 无仓位卖出应被拒绝
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let sell = Order::new_market(broker.next_order_id(), OrderSide::Sell, 10);
    broker.submit_order(sell, 0);

    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "无仓位时卖出应被拒绝");

    let notifications = broker.drain_notifications();
    let rejected = notifications.iter().any(|n| matches!(n, OrderNotification::OrderRejected(_)));
    assert!(rejected, "应有订单被拒绝的通知（仓位不足）");
}

// ========== 修复 #2: Bracket 订单 OCO 取消测试 ==========

#[test]
fn test_bracket_oco_take_profit_cancels_stop_loss() {
    // 当止盈单成交后，止损单应被 OCO 取消（而不是被 Rejected）
    let mut broker = setup_broker(20000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    // Bracket: Buy Limit @ 98, Take Profit @ 110, Stop Loss @ 92
    let (_entry_id, _tp_id, _sl_id) =
        broker.bracket_order(OrderSide::Buy, 10, 98.0, 110.0, 92.0, 0);

    // bar2: low=96 <= 98 -> entry 成交
    let bar2 = make_bar((2024, 1, 2), 99.0, 102.0, 96.0, 100.0);
    broker.next_bar(&bar2, 0);
    broker.drain_notifications(); // 清除入场通知

    // bar3: high=112 >= 110 -> Take Profit 应触发
    let bar3 = make_bar((2024, 1, 3), 105.0, 112.0, 104.0, 111.0);
    broker.next_bar(&bar3, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "Take Profit 应触发并平仓");

    let notifications = broker.drain_notifications();
    // 检查是否有订单被取消（OCO 逻辑）
    let canceled = notifications.iter()
        .filter(|n| matches!(n, OrderNotification::OrderCanceled(_)))
        .count();
    assert!(canceled >= 1, "TP 成交后 SL 应被 OCO 取消，实际取消数: {}", canceled);

    // 确认没有被 Rejected 的订单（之前 bug 是 SL 被 Rejected 而不是 Canceled）
    let rejected = notifications.iter()
        .filter(|n| matches!(n, OrderNotification::OrderRejected(_)))
        .count();
    assert_eq!(rejected, 0, "OCO 取消不应产生 Rejected 通知");
}

// ========== 修复 #6: size=0 订单拒绝测试 ==========

#[test]
fn test_zero_size_order_rejected() {
    // size=0 的订单应被拒绝，不应产生任何仓位变化或 Trade 记录
    let mut broker = setup_broker(10000.0);
    let bar1 = make_bar((2024, 1, 1), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar1, 0);

    let buy = Order::new_market(broker.next_order_id(), OrderSide::Buy, 0);
    broker.submit_order(buy, 0);

    let bar2 = make_bar((2024, 1, 2), 100.0, 105.0, 95.0, 102.0);
    broker.next_bar(&bar2, 0);

    let pos = broker.get_position(0);
    assert_eq!(pos.size, 0, "size=0 的订单不应改变仓位");

    let notifications = broker.drain_notifications();
    let rejected = notifications.iter().any(|n| matches!(n, OrderNotification::OrderRejected(_)));
    assert!(rejected, "size=0 的订单应产生 Rejected 通知");

    let trades = broker.get_trades();
    assert_eq!(trades.len(), 0, "size=0 的订单不应产生 Trade 记录");
}
