use crate::core::Bar;

/// DataFeed trait：数据源的统一接口
/// 所有数据源（CSV、实时行情等）都实现此 trait
pub trait DataFeed {
    /// 获取下一根 K 线，若数据耗尽返回 None
    fn next_bar(&mut self) -> Option<Bar>;

    /// 重置数据源到初始状态（用于多次回测）
    fn reset(&mut self);

    /// 数据总量
    fn len(&self) -> usize;

    /// 是否为空
    fn is_empty(&self) -> bool;
}
