/// Phase 2 指标综合测试

use backtrader_rust::core::Bar;
use backtrader_rust::indicators::{
    Indicator, SMA, EMA, RSI, MACD, BollingerBands, ATR, Stochastic,
    CrossOver, CrossSignal, ChainedIndicator,
};
use chrono::NaiveDate;

// ========== 辅助函数 ==========

fn make_bar(o: f64, h: f64, l: f64, c: f64) -> Bar {
    let dt = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    Bar::new(dt, o, h, l, c, 1000000.0, 0.0)
}

fn make_bar_dt(day: u32, o: f64, h: f64, l: f64, c: f64) -> Bar {
    let dt = NaiveDate::from_ymd_opt(2024, 1, day).unwrap().and_hms_opt(0, 0, 0).unwrap();
    Bar::new(dt, o, h, l, c, 1000000.0, 0.0)
}

// ========== EMA 测试 ==========

#[test]
fn test_ema_smoothing_factor() {
    // 验证平滑因子 = 2 / (period + 1)
    let mut ema = EMA::new(9);
    // smoothing = 2 / 10 = 0.2
    let prices = vec![22.27, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24];
    for p in &prices[..8] {
        assert!(ema.next(*p).is_none());
    }
    // 第9个值：SMA = sum/9
    let sma_val: f64 = prices.iter().sum::<f64>() / 9.0;
    let v = ema.next(prices[8]).unwrap();
    assert!((v - sma_val).abs() < 1e-6, "第一个EMA应等于SMA: {} vs {}", v, sma_val);

    // 第10个值：EMA = (new - prev) * 0.2 + prev
    let new_price = 22.37;
    let expected = (new_price - v) * 0.2 + v;
    let v2 = ema.next(new_price).unwrap();
    assert!((v2 - expected).abs() < 1e-9, "EMA递推: {} vs {}", v2, expected);
}

#[test]
fn test_ema_min_period() {
    let ema = EMA::new(10);
    assert_eq!(ema.min_period(), 10);
    assert!(!ema.is_ready());
}

#[test]
fn test_ema_empty_input() {
    let mut ema = EMA::new(5);
    assert!(ema.next(1.0).is_none());
    assert!(!ema.is_ready());
}

// ========== RSI 测试 ==========

#[test]
fn test_rsi_wilder_smoothing() {
    // 验证 Wilder 平滑递推
    let mut rsi = RSI::new(3);
    let prices = vec![44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10];

    // 喂入所有价格
    let mut results = Vec::new();
    for p in &prices {
        if let Some(v) = rsi.next(*p) {
            results.push(v);
        }
    }

    // 第一个 RSI 在第4个价格（period+1=4）产生
    assert!(!results.is_empty());

    // 验证 Wilder 平滑：第二个RSI
    // 初始变化: +0.34, -0.25, -0.48 -> avg_gain = 0.34/3, avg_loss = 0.73/3
    let avg_gain1 = 0.34 / 3.0;
    let avg_loss1 = (0.25 + 0.48) / 3.0;
    let rsi1 = 100.0 - 100.0 / (1.0 + avg_gain1 / avg_loss1);
    assert!((results[0] - rsi1).abs() < 1e-6, "RSI第一个值: {} vs {}", results[0], rsi1);
}

#[test]
fn test_rsi_100_special_case() {
    // 全部上涨 -> RSI = 100
    let mut rsi = RSI::new(5);
    rsi.next(10.0);
    for i in 1..=5 {
        rsi.next(10.0 + i as f64);
    }
    assert!(rsi.is_ready());
    let v = rsi.value().unwrap();
    assert!((v - 100.0).abs() < 1e-9, "全涨RSI应为100: {}", v);
}

#[test]
fn test_rsi_min_period() {
    let rsi = RSI::new(14);
    assert_eq!(rsi.min_period(), 15);
}

// ========== MACD 测试 ==========

#[test]
fn test_macd_three_output_lines() {
    let mut macd = MACD::new(5, 10, 3);
    // min_period = 10 + 3 - 1 = 12
    let prices: Vec<f64> = (1..=15).map(|i| 100.0 + (i as f64) * 0.5).collect();

    let mut last_result = None;
    for p in &prices {
        if let Some(v) = macd.next_multi(*p) {
            last_result = Some(v);
        }
    }

    let result = last_result.unwrap();
    assert_eq!(result.len(), 3);
    // histogram = macd - signal
    assert!((result[2] - (result[0] - result[1])).abs() < 1e-9);
}

#[test]
fn test_macd_min_period_correct() {
    let macd = MACD::new(12, 26, 9);
    assert_eq!(macd.min_period(), 34);

    let mut macd = MACD::new(12, 26, 9);
    // 前33个值应该返回 None
    for i in 0..33 {
        let v = macd.next(100.0 + i as f64 * 0.1);
        assert!(v.is_none(), "第{}个值应返回None", i + 1);
    }
    // 第34个值应该产生输出
    let v = macd.next(110.0);
    assert!(v.is_some(), "第34个值应产生输出");
}

// ========== Bollinger Bands 测试 ==========

#[test]
fn test_bollinger_upper_lower_symmetry() {
    let mut bb = BollingerBands::new(10, 2.0);
    let prices = vec![10.0, 11.0, 10.5, 12.0, 11.5, 13.0, 12.5, 14.0, 13.5, 15.0];
    let mut last = None;
    for p in &prices {
        if let Some(v) = bb.next_multi(*p) {
            last = Some(v);
        }
    }
    let result = last.unwrap();
    let mid = result[0];
    let upper = result[1];
    let lower = result[2];
    // 上下带关于中线对称
    assert!((upper - mid - (mid - lower)).abs() < 1e-9);
}

#[test]
fn test_bollinger_constant_price() {
    // 恒定价格 -> 标准差为0 -> 上下带等于中线
    let mut bb = BollingerBands::new(5, 2.0);
    for _ in 0..5 {
        bb.next_multi(100.0);
    }
    let (mid, upper, lower) = bb.values().unwrap();
    assert!((mid - 100.0).abs() < 1e-9);
    assert!((upper - 100.0).abs() < 1e-9);
    assert!((lower - 100.0).abs() < 1e-9);
}

// ========== ATR 测试 ==========

#[test]
fn test_atr_true_range_calculation() {
    let mut atr = ATR::new(3);
    // bar1: H=10, L=5, C=8 -> TR = 10-5 = 5 (无前收)
    let b1 = make_bar_dt(1, 7.0, 10.0, 5.0, 8.0);
    assert!(atr.next_bar(&b1).is_none());

    // bar2: H=12, L=6, C=9 -> TR = max(6, |12-8|=4, |6-8|=2) = 6
    let b2 = make_bar_dt(2, 9.0, 12.0, 6.0, 9.0);
    assert!(atr.next_bar(&b2).is_none());

    // bar3: H=11, L=7, C=10 -> TR = max(4, |11-9|=2, |7-9|=2) = 4
    let b3 = make_bar_dt(3, 10.0, 11.0, 7.0, 10.0);
    let v = atr.next_bar(&b3).unwrap();
    // ATR = (5+6+4)/3 = 5.0
    assert!((v - 5.0).abs() < 1e-9);

    // bar4: H=15, L=9, C=14 -> TR = max(6, |15-10|=5, |9-10|=1) = 6
    // Wilder: ATR = (5.0 * 2 + 6) / 3 = 16/3 ≈ 5.333
    let b4 = make_bar_dt(4, 11.0, 15.0, 9.0, 14.0);
    let v2 = atr.next_bar(&b4).unwrap();
    assert!((v2 - 16.0 / 3.0).abs() < 1e-6, "ATR Wilder平滑: {}", v2);
}

// ========== Stochastic 测试 ==========

#[test]
fn test_stochastic_k_d_range() {
    let mut stoch = Stochastic::new(5, 3);
    // 生成一些价格数据
    let bars: Vec<Bar> = (0..10)
        .map(|i| {
            let base = 100.0 + (i as f64) * 2.0;
            make_bar_dt(i as u32 + 1, base, base + 3.0, base - 2.0, base + 1.0)
        })
        .collect();

    for bar in &bars {
        if let Some(vals) = stoch.next_bar_multi(bar) {
            assert!(vals[0] >= 0.0 && vals[0] <= 100.0, "%K 越界: {}", vals[0]);
            assert!(vals[1] >= 0.0 && vals[1] <= 100.0, "%D 越界: {}", vals[1]);
        }
    }
}

#[test]
fn test_stochastic_at_lowest() {
    let mut stoch = Stochastic::new(3, 2);
    // min_period = 3 + 2 - 1 = 4
    let b1 = make_bar(10.0, 15.0, 8.0, 12.0);
    let b2 = make_bar(11.0, 14.0, 9.0, 11.0);
    let b3 = make_bar(12.0, 13.0, 8.0, 8.0); // close = lowest_low
    let b4 = make_bar(11.0, 14.0, 7.0, 7.0); // close = lowest_low
    stoch.next_bar_multi(&b1);
    stoch.next_bar_multi(&b2);
    stoch.next_bar_multi(&b3);
    let result = stoch.next_bar_multi(&b4);
    assert!(result.is_some(), "第4根bar应产生输出");
    let vals = result.unwrap();
    assert!((vals[0] - 0.0).abs() < 1e-9, "%K应为0: {}", vals[0]);
}

// ========== CrossOver 测试 ==========

#[test]
fn test_crossover_sequence() {
    let mut co = CrossOver::new();
    // fast < slow -> 无交叉
    assert_eq!(co.next(1.0, 5.0), None);
    // fast > slow -> CrossUp
    assert_eq!(co.next(6.0, 5.0), Some(CrossSignal::CrossUp));
    // fast 继续大于 slow -> 无交叉
    assert_eq!(co.next(7.0, 5.0), None);
    // fast < slow -> CrossDown
    assert_eq!(co.next(4.0, 5.0), Some(CrossSignal::CrossDown));
    // fast 继续小于 slow -> 无交叉
    assert_eq!(co.next(3.0, 5.0), None);
    // fast > slow -> CrossUp
    assert_eq!(co.next(6.0, 5.0), Some(CrossSignal::CrossUp));
}

// ========== ChainedIndicator 测试 ==========

#[test]
fn test_chained_sma_of_rsi() {
    // SMA(3) of RSI(3)
    // RSI min_period = 4, SMA min_period = 3
    // ChainedIndicator min_period = 4 + 3 - 1 = 6
    let rsi = RSI::new(3);
    let sma = SMA::new(3);
    let mut chained = ChainedIndicator::new(rsi, sma);

    assert_eq!(chained.min_period(), 6);
    assert_eq!(chained.name(), "SMA(RSI)");

    // 前5个值应返回 None（RSI需要4个点才出第一个值，SMA还需要额外2个RSI值）
    let prices = vec![44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10];
    let mut results = Vec::new();
    for p in &prices {
        if let Some(v) = chained.next(*p) {
            results.push(v);
        }
    }
    // 第6个价格开始产生 SMA(RSI) 值
    assert!(!results.is_empty(), "应产生至少一个值");
    assert!(chained.is_ready());
}

#[test]
fn test_chained_reset() {
    let rsi = RSI::new(3);
    let sma = SMA::new(3);
    let mut chained = ChainedIndicator::new(rsi, sma);

    for i in 0..10 {
        chained.next(100.0 + i as f64);
    }
    assert!(chained.is_ready());
    chained.reset();
    assert!(!chained.is_ready());
}

// ========== 多输出线 trait 方法测试 ==========

#[test]
fn test_output_count_default() {
    let sma = SMA::new(5);
    assert_eq!(sma.output_count(), 1);

    let ema = EMA::new(10);
    assert_eq!(ema.output_count(), 1);
}

#[test]
fn test_next_multi_default() {
    let mut sma = SMA::new(3);
    sma.next(1.0);
    sma.next(2.0);
    let result = sma.next_multi(3.0).unwrap();
    assert_eq!(result.len(), 1);
    assert!((result[0] - 2.0).abs() < 1e-9);
}

#[test]
fn test_next_bar_default() {
    // 默认 next_bar 应该用 close 调用 next
    let mut sma = SMA::new(2);
    let bar1 = make_bar(10.0, 12.0, 8.0, 10.0);
    let bar2 = make_bar(11.0, 13.0, 9.0, 20.0);
    assert!(sma.next_bar(&bar1).is_none());
    let v = sma.next_bar(&bar2).unwrap();
    // SMA = (10 + 20) / 2 = 15 (使用 close 价格)
    assert!((v - 15.0).abs() < 1e-9);
}
