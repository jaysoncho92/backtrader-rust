/// Sizer trait：定义仓位大小的计算接口
///
/// 根据当前资金、价格和交易方向，计算应下单的手数
pub trait Sizer {
    /// 计算下单手数
    /// - cash: 当前可用现金
    /// - price: 当前价格
    /// - is_buy: 是否买入
    fn get_size(&self, cash: f64, price: f64, is_buy: bool) -> i64;
}
