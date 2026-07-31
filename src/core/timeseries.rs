use std::ops::{Add, Div, Index, Mul, Sub};

/// TimeSeries<T>: 泛型时序容器
/// 索引约定: ts[0] = 最新值, ts[-1] = 前一个值, ts[-2] = 前两个值...
/// 这与 Python backtrader 的索引语义一致
#[derive(Debug, Clone)]
pub struct TimeSeries<T> {
    data: Vec<T>,
}

impl<T> TimeSeries<T> {
    /// 创建空的 TimeSeries
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// 追加一个新值（最新的 bar）
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// 返回已存储的元素个数
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 获取最新值（最后一个元素）的引用
    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }

    /// 按 ago 偏移获取历史值：ago=0 最新, ago=1 前一根, ...
    pub fn get(&self, ago: isize) -> Option<&T> {
        if self.data.is_empty() {
            return None;
        }
        let len = self.data.len() as isize;
        // ago=0 -> index len-1, ago=-1 -> index len-2 (但语义上 ago 应该是非负数表示向前几个)
        // 根据需求: ts[0] = 最新值 -> data[len-1], ts[-1] = 前一个 -> data[len-2]
        // Index<isize> 用 0 和负数访问；get() 方法用非负 ago 表示历史偏移
        let idx = (len - 1 - ago) as usize;
        if ago < 0 || ago >= len {
            None
        } else {
            self.data.get(idx)
        }
    }
}

impl<T> Default for TimeSeries<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Index<isize> 实现：ts[0] = 最新值, ts[-1] = 前一个值
/// index 0 -> data[len-1], index -1 -> data[len-2], index -2 -> data[len-3]...
impl<T> Index<isize> for TimeSeries<T> {
    type Output = T;

    fn index(&self, index: isize) -> &Self::Output {
        let len = self.data.len() as isize;
        assert!(!self.data.is_empty(), "TimeSeries 为空，无法索引");
        assert!(index <= 0, "TimeSeries 索引必须 <= 0，收到 {}", index);
        let real_idx = (len - 1 + index) as usize;
        &self.data[real_idx]
    }
}

// ========== f64 TimeSeries 的逐元素算术运算 ==========

impl Add for TimeSeries<f64> {
    type Output = TimeSeries<f64>;
    fn add(self, rhs: Self) -> Self::Output {
        let len = self.data.len().min(rhs.data.len());
        let mut result = TimeSeries::new();
        for i in 0..len {
            result.push(self.data[i] + rhs.data[i]);
        }
        result
    }
}

impl Sub for TimeSeries<f64> {
    type Output = TimeSeries<f64>;
    fn sub(self, rhs: Self) -> Self::Output {
        let len = self.data.len().min(rhs.data.len());
        let mut result = TimeSeries::new();
        for i in 0..len {
            result.push(self.data[i] - rhs.data[i]);
        }
        result
    }
}

impl Mul for TimeSeries<f64> {
    type Output = TimeSeries<f64>;
    fn mul(self, rhs: Self) -> Self::Output {
        let len = self.data.len().min(rhs.data.len());
        let mut result = TimeSeries::new();
        for i in 0..len {
            result.push(self.data[i] * rhs.data[i]);
        }
        result
    }
}

impl Div for TimeSeries<f64> {
    type Output = TimeSeries<f64>;
    fn div(self, rhs: Self) -> Self::Output {
        let len = self.data.len().min(rhs.data.len());
        let mut result = TimeSeries::new();
        for i in 0..len {
            result.push(self.data[i] / rhs.data[i]);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeseries_index() {
        let mut ts = TimeSeries::new();
        ts.push(10.0);
        ts.push(20.0);
        ts.push(30.0);
        // ts[0] = 最新 = 30, ts[-1] = 20, ts[-2] = 10
        assert_eq!(ts[0], 30.0);
        assert_eq!(ts[-1], 20.0);
        assert_eq!(ts[-2], 10.0);
    }

    #[test]
    fn test_timeseries_get() {
        let mut ts = TimeSeries::new();
        ts.push(100.0);
        ts.push(200.0);
        assert_eq!(ts.get(0), Some(&200.0));
        assert_eq!(ts.get(1), Some(&100.0));
        assert_eq!(ts.get(2), None);
    }
}
