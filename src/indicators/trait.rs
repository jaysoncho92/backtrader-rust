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
}
