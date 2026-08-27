/// 多时间框架策略示例
/// 使用 Resampler 将日线数据聚合为周线
/// 周线 SMA(10) 判断趋势方向，日线 SMA(5) 找入场点

use backtrader_rust::analyzers::{DrawDown, SharpeRatio, TimeReturn, TradeAnalyzer};
use backtrader_rust::core::TimeFrame;
use backtrader_rust::engine::CerebroBuilder;
use backtrader_rust::feeds::{CsvFeed, DataFeed, Resampler};
use backtrader_rust::indicators::{Indicator, SMA};
use backtrader_rust::strategy::{Context, Strategy};

/// 多时间框架策略
/// - 周线 SMA(10) 判断大趋势方向
/// - 日线 SMA(5) 寻找入场时机
/// - 周线上升 + 日线 SMA 上穿 -> 买入
/// - 周线下降 或 日线 SMA 下穿 -> 卖出
struct MultiTimeframeStrategy {
    daily_sma: Option<SMA>,
    weekly_sma: Option<SMA>,
    prev_daily_sma: Option<f64>,
    prev_weekly_close: Option<f64>,
}

impl MultiTimeframeStrategy {
    fn new() -> Self {
        Self {
            daily_sma: Some(SMA::new(5)),
            weekly_sma: Some(SMA::new(10)),
            prev_daily_sma: None,
            prev_weekly_close: None,
        }
    }
}

impl Strategy for MultiTimeframeStrategy {
    fn init(&mut self, _ctx: &mut Context) {
        println!("多时间框架策略初始化完成");
    }

    fn next(&mut self, ctx: &mut Context) {
        let daily_data = ctx.data(0);
        let weekly_data = ctx.data(1);

        if daily_data.is_empty() {
            return;
        }

        let daily_bar = &daily_data[0isize];
        let daily_close = daily_bar.close;
        let datetime = daily_bar.datetime;
        let current_bar = ctx.current_bar;

        // 更新日线 SMA
        let daily_sma_val = self.daily_sma.as_mut().unwrap().next(daily_close);
        let Some(daily_sma) = daily_sma_val else { return };

        // 检查周线是否有新数据
        let weekly_trend_up = if !weekly_data.is_empty() {
            let weekly_bar = &weekly_data[0isize];
            let weekly_close = weekly_bar.close;

            // 更新周线 SMA（仅当周线数据变化时）
            let weekly_sma_val = if Some(weekly_close) != self.prev_weekly_close {
                self.prev_weekly_close = Some(weekly_close);
                self.weekly_sma.as_mut().unwrap().next(weekly_close)
            } else {
                // 周线未更新，使用上次的 SMA 值
                None
            };

            // 判断周线趋势：周线收盘价 > 周线 SMA 则趋势向上
            if let Some(ws) = weekly_sma_val {
                weekly_close > ws
            } else {
                // 如果周线 SMA 尚未就绪，使用前值判断
                self.prev_weekly_close.map(|_c| {
                    if !weekly_data.is_empty() {
                        let wb = &weekly_data[0isize];
                        wb.close > wb.open // 简单判断：收盘价 > 开盘价
                    } else {
                        false
                    }
                }).unwrap_or(false)
            }
        } else {
            false
        };

        let pos_size = ctx.position(0).size;
        let has_position = pos_size > 0;

        // 交易逻辑
        if let Some(prev_daily) = self.prev_daily_sma {
            let daily_cross_up = prev_daily <= daily_sma && daily_close > daily_sma;
            let daily_cross_down = prev_daily >= daily_sma && daily_close < daily_sma;

            // 买入条件：周线趋势向上 且 日线价格上穿日线 SMA
            if weekly_trend_up && daily_cross_up && !has_position {
                let cash_available = ctx.cash() * 0.95;
                let size = (cash_available / daily_close) as i64;
                if size > 0 {
                    ctx.buy(0, size);
                    println!(
                        "[买入] bar={} dt={} close={:.2} daily_sma={:.2} 周线趋势=上升",
                        current_bar, datetime, daily_close, daily_sma
                    );
                }
            }
            // 卖出条件：周线趋势向下 或 日线价格下穿日线 SMA
            else if has_position && (!weekly_trend_up || daily_cross_down) {
                ctx.sell(0, pos_size);
                let reason = if !weekly_trend_up {
                    "周线趋势下降"
                } else {
                    "日线下穿 SMA"
                };
                println!(
                    "[卖出] bar={} dt={} close={:.2} daily_sma={:.2} 原因={}",
                    current_bar, datetime, daily_close, daily_sma, reason
                );
            }
        }

        self.prev_daily_sma = Some(daily_sma);
    }

    fn stop(&mut self, ctx: &mut Context) {
        println!("\n回测结束!");
        println!("最终现金: {:.2}", ctx.cash());
        println!("最终组合价值: {:.2}", ctx.portfolio_value(0));
    }
}

fn main() {
    println!("=== 多时间框架策略示例 ===\n");

    // 创建日线数据源
    let daily_feed = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载日线数据");
    println!("加载日线数据: {} 根 K 线", daily_feed.len());

    // 创建周线数据源（通过 Resampler 聚合日线）
    let weekly_feed_source = CsvFeed::new("sample_data/orcl-2014.txt")
        .expect("无法加载日线数据（用于周线重采样）");
    let weekly_feed = Resampler::new(weekly_feed_source, TimeFrame::Weeks);
    println!("周线重采样器已创建");

    // 创建策略
    let strategy = MultiTimeframeStrategy::new();

    // 构建并运行回测（日线为主时钟，周线为辅助）
    let result = CerebroBuilder::new()
        .cash(10000.0)
        .commission(0.005)
        .add_data(Box::new(daily_feed))   // 数据源 0：日线
        .add_data(Box::new(weekly_feed))  // 数据源 1：周线
        .add_strategy(Box::new(strategy))
        .add_analyzer(Box::new(TimeReturn::new()))
        .add_analyzer(Box::new(SharpeRatio::new()))
        .add_analyzer(Box::new(DrawDown::new()))
        .add_analyzer(Box::new(TradeAnalyzer::new()))
        .run();

    // 打印结果
    println!("\n========== 回测结果 ==========");
    println!("处理的 Bar 数: {}", result.bars_processed);
    println!("最终组合价值: {:.2}", result.final_value);
    println!("总收益率:     {:.2}%", result.total_return);
    println!("交易次数:     {}", result.trades.len());

    result.print_summary();
}
