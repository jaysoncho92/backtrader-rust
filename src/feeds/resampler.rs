use chrono::{Datelike, NaiveDateTime};

use crate::core::{Bar, TimeFrame};
use super::DataFeed;

/// 数据重采样器：将低时间框架数据聚合为高时间框架
///
/// 支持日线→周线、日线→月线的聚合转换
pub struct Resampler<F: DataFeed> {
    inner: F,
    target_timeframe: TimeFrame,
    current_bar: Option<Bar>,  // 正在聚合的 bar
    pending_bar: Option<Bar>,  // 聚合完成待返回的 bar
    current_group: Option<BarGroup>, // 当前分组标识
    done: bool,                // 内部 feed 是否已耗尽
}

/// 用于判断 bar 属于哪个聚合分组
#[derive(Debug, Clone, PartialEq, Eq)]
enum BarGroup {
    /// ISO (year, week)
    Week(i32, u32),
    /// (year, month)
    Month(i32, u32),
}

impl<F: DataFeed> Resampler<F> {
    /// 创建 Resampler
    /// - inner: 原始低时间框架数据源
    /// - target: 目标时间框架（Weeks 或 Months）
    pub fn new(inner: F, target: TimeFrame) -> Self {
        assert!(
            target == TimeFrame::Weeks || target == TimeFrame::Months,
            "Resampler 目前仅支持 Weeks 和 Months 目标时间框架"
        );
        Self {
            inner,
            target_timeframe: target,
            current_bar: None,
            pending_bar: None,
            current_group: None,
            done: false,
        }
    }

    /// 获取 bar 所属的分组
    fn get_group(&self, dt: &NaiveDateTime) -> BarGroup {
        match self.target_timeframe {
            TimeFrame::Weeks => {
                let iso = dt.iso_week();
                BarGroup::Week(iso.year(), iso.week())
            }
            TimeFrame::Months => {
                BarGroup::Month(dt.year(), dt.month())
            }
            _ => unreachable!(),
        }
    }

    /// 将新 bar 聚合到当前 bar
    fn aggregate(current: &mut Bar, new_bar: &Bar) {
        // high = max, low = min, close = 最新, volume = sum
        if new_bar.high > current.high {
            current.high = new_bar.high;
        }
        if new_bar.low < current.low {
            current.low = new_bar.low;
        }
        current.close = new_bar.close;
        current.volume += new_bar.volume;
        // open 和 datetime 保持第一根 bar 的值不变
    }

    /// 从聚合数据构建一根完成的 Bar
    fn finalize_bar(bar: &Bar) -> Bar {
        bar.clone()
    }
}

impl<F: DataFeed> DataFeed for Resampler<F> {
    fn next_bar(&mut self) -> Option<Bar> {
        // 如果有待返回的 bar，先返回它
        if let Some(bar) = self.pending_bar.take() {
            return Some(bar);
        }

        // 从内部 feed 读取数据
        loop {
            let next = if self.done {
                None
            } else {
                self.inner.next_bar()
            };

            match next {
                Some(bar) => {
                    let group = self.get_group(&bar.datetime);

                    if self.current_group.is_none() {
                        // 第一根 bar，初始化
                        self.current_group = Some(group);
                        self.current_bar = Some(bar);
                    } else if self.current_group.as_ref() == Some(&group) {
                        // 同一分组，聚合
                        if let Some(ref mut current) = self.current_bar {
                            Self::aggregate(current, &bar);
                        }
                    } else {
                        // 新分组：完成当前 bar，开始新聚合
                        let completed = self.current_bar.as_ref().map(Self::finalize_bar);
                        self.current_group = Some(group);
                        self.current_bar = Some(bar);
                        return completed;
                    }
                }
                None => {
                    // 内部 feed 耗尽
                    self.done = true;
                    // 返回最后一根聚合 bar
                    return self.current_bar.take();
                }
            }
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.current_bar = None;
        self.pending_bar = None;
        self.current_group = None;
        self.done = false;
    }

    fn len(&self) -> usize {
        // 无法预知聚合后的数量，返回 0 作为近似
        0
    }

    fn is_empty(&self) -> bool {
        self.current_bar.is_none() && self.pending_bar.is_none() && self.done
    }
}
