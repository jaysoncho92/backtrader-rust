use crate::core::Bar;

/// Indicator trait：所有技术指标的统一接口
pub trait Indicator {
    /// 指标名称
    fn name(&self) -> &str;

    /// 推送一个新值，返回指标计算结果（未就绪时返回 None）
    fn next(&mut self, value: f64) -> Option<f64>;

    /// 指标所需的最小数据周期
    fn min_period(&self) -> usize;

    /// 指标是否已就绪（收集了足够的数据）
    fn is_ready(&self) -> bool;

    /// 重置指标状态
    fn reset(&mut self);

    // ========== Phase 2 扩展方法（均有默认实现，保持向后兼容） ==========

    /// 输出线数量（默认 1 条）
    fn output_count(&self) -> usize {
        1
    }

    /// 多输出线版本：推送一个新值，返回多条输出线的值
    fn next_multi(&mut self, value: f64) -> Option<Vec<f64>> {
        self.next(value).map(|v| vec![v])
    }

    /// OHLC 输入：推送一根完整 Bar，返回单条输出线值（默认只用 close）
    fn next_bar(&mut self, bar: &Bar) -> Option<f64> {
        self.next(bar.close)
    }

    /// OHLC 输入 + 多输出线：推送一根完整 Bar，返回多条输出线值
    fn next_bar_multi(&mut self, bar: &Bar) -> Option<Vec<f64>> {
        self.next_bar(bar).map(|v| vec![v])
    }
}

/// ChainedIndicator：将两个指标串联，inner 的输出作为 outer 的输入
pub struct ChainedIndicator<I1: Indicator, I2: Indicator> {
    inner: I1,
    outer: I2,
    /// 缓存的组合名称
    chained_name: String,
}

impl<I1: Indicator, I2: Indicator> ChainedIndicator<I1, I2> {
    /// 创建串联指标：inner 先处理输入，其输出传给 outer
    pub fn new(inner: I1, outer: I2) -> Self {
        let chained_name = format!("{}({})", outer.name(), inner.name());
        Self {
            inner,
            outer,
            chained_name,
        }
    }

    /// 获取内部指标的引用
    pub fn inner(&self) -> &I1 {
        &self.inner
    }

    /// 获取外部指标的引用
    pub fn outer(&self) -> &I2 {
        &self.outer
    }
}

impl<I1: Indicator, I2: Indicator> Indicator for ChainedIndicator<I1, I2> {
    fn name(&self) -> &str {
        &self.chained_name
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        // 先让 inner 处理，如果有输出则传给 outer
        match self.inner.next(value) {
            Some(v) => self.outer.next(v),
            None => None,
        }
    }

    fn min_period(&self) -> usize {
        // 串联指标的最小周期 = inner 的最小周期 + outer 的最小周期 - 1
        // 因为 inner 输出第一个值后，outer 还需要 min_period-1 个额外值
        self.inner.min_period() + self.outer.min_period() - 1
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready() && self.outer.is_ready()
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.outer.reset();
    }
}
