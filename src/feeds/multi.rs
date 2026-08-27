use crate::core::Bar;
use super::DataFeed;

/// 多数据源管理器：同步多个数据源的时间轴
///
/// 以第一个数据源为主时钟，其他数据源按时间对齐
pub struct MultiDataFeed {
    feeds: Vec<Box<dyn DataFeed>>,
    bars: Vec<Vec<Bar>>,      // 每个 feed 的全部数据（预加载）
    current_index: usize,      // 主时钟当前索引
    loaded: bool,              // 是否已加载所有数据
}

impl MultiDataFeed {
    /// 创建空的多数据源管理器
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            bars: Vec::new(),
            current_index: 0,
            loaded: false,
        }
    }

    /// 添加一个数据源
    pub fn add_feed(&mut self, feed: Box<dyn DataFeed>) {
        self.feeds.push(feed);
        self.bars.push(Vec::new());
    }

    /// 获取数据源数量
    pub fn feed_count(&self) -> usize {
        self.feeds.len()
    }

    /// 预加载所有 feed 的数据到内存
    fn load_all(&mut self) {
        if self.loaded {
            return;
        }
        for (idx, feed) in self.feeds.iter_mut().enumerate() {
            feed.reset();
            let mut all_bars = Vec::new();
            while let Some(bar) = feed.next_bar() {
                all_bars.push(bar);
            }
            self.bars[idx] = all_bars;
        }
        self.loaded = true;
    }

    /// 返回所有 feed 在同一时间点的 bars
    /// 以第一个 feed 的当前 bar datetime 为基准，
    /// 其他 feed 找到时间最接近的 bar
    pub fn next_bars(&mut self) -> Vec<Option<Bar>> {
        self.load_all();

        if self.bars.is_empty() || self.bars[0].is_empty() {
            return vec![None; self.feeds.len()];
        }

        if self.current_index >= self.bars[0].len() {
            return vec![None; self.feeds.len()];
        }

        let ref_bar = &self.bars[0][self.current_index];
        let ref_dt = ref_bar.datetime;

        let mut result = Vec::with_capacity(self.feeds.len());

        // 第一个 feed 直接返回当前索引的 bar
        result.push(Some(ref_bar.clone()));

        // 其他 feed 找时间最接近的 bar
        for feed_idx in 1..self.bars.len() {
            let feed_bars = &self.bars[feed_idx];
            if feed_bars.is_empty() {
                result.push(None);
                continue;
            }

            // 二分查找最接近 ref_dt 的 bar
            let closest = find_closest_bar(feed_bars, ref_dt);
            result.push(closest);
        }

        self.current_index += 1;
        result
    }
}

impl Default for MultiDataFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl DataFeed for MultiDataFeed {
    /// next_bar 返回第一个 feed（主时钟）的下一根 bar
    fn next_bar(&mut self) -> Option<Bar> {
        self.load_all();

        if self.bars.is_empty() || self.bars[0].is_empty() {
            return None;
        }

        if self.current_index < self.bars[0].len() {
            let bar = self.bars[0][self.current_index].clone();
            self.current_index += 1;
            Some(bar)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.current_index = 0;
        // 同时重置所有内部 feed
        for feed in self.feeds.iter_mut() {
            feed.reset();
        }
        // 重新加载
        self.loaded = false;
    }

    fn len(&self) -> usize {
        if !self.loaded {
            // 尝试获取第一个 feed 的长度
            if !self.feeds.is_empty() {
                return self.feeds[0].len();
            }
            return 0;
        }
        if self.bars.is_empty() {
            0
        } else {
            self.bars[0].len()
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 在有序 bar 数组中查找时间最接近 target_dt 的 bar
fn find_closest_bar(bars: &[Bar], target_dt: chrono::NaiveDateTime) -> Option<Bar> {
    if bars.is_empty() {
        return None;
    }

    // 线性扫描找到最接近的（bar 按时间升序）
    let mut best_idx = 0;
    let mut best_diff = (bars[0].datetime - target_dt).num_seconds().unsigned_abs();

    for (i, bar) in bars.iter().enumerate().skip(1) {
        let diff = (bar.datetime - target_dt).num_seconds().unsigned_abs();
        if diff < best_diff {
            best_diff = diff;
            best_idx = i;
        } else if bar.datetime > target_dt {
            // 已超过目标时间，后面的更远，停止搜索
            break;
        }
    }

    Some(bars[best_idx].clone())
}
