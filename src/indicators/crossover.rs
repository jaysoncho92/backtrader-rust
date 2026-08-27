/// CrossSignal：交叉信号枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossSignal {
    /// 上穿：fast 从下方穿越 slow
    CrossUp,
    /// 下穿：fast 从上方穿越 slow
    CrossDown,
}

/// CrossOver：检测两条线的交叉事件
/// 需要记住前一对值来判断交叉方向
pub struct CrossOver {
    /// 前一对 (fast, slow) 值
    prev_fast: Option<f64>,
    prev_slow: Option<f64>,
}

impl CrossOver {
    pub fn new() -> Self {
        Self {
            prev_fast: None,
            prev_slow: None,
        }
    }

    /// 推送一对新值，返回交叉信号（如果发生交叉）
    pub fn next(&mut self, fast: f64, slow: f64) -> Option<CrossSignal> {
        let signal = match (self.prev_fast, self.prev_slow) {
            (Some(pf), Some(ps)) => {
                if pf <= ps && fast > slow {
                    Some(CrossSignal::CrossUp)
                } else if pf >= ps && fast < slow {
                    Some(CrossSignal::CrossDown)
                } else {
                    None
                }
            }
            _ => None,
        };

        self.prev_fast = Some(fast);
        self.prev_slow = Some(slow);
        signal
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.prev_fast = None;
        self.prev_slow = None;
    }
}

impl Default for CrossOver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossover_up() {
        let mut co = CrossOver::new();
        // 第一次：无交叉
        assert_eq!(co.next(1.0, 2.0), None);
        // fast 从下方穿越 slow
        assert_eq!(co.next(3.0, 2.0), Some(CrossSignal::CrossUp));
    }

    #[test]
    fn test_crossover_down() {
        let mut co = CrossOver::new();
        assert_eq!(co.next(3.0, 2.0), None);
        // fast 从上方穿越 slow
        assert_eq!(co.next(1.0, 2.0), Some(CrossSignal::CrossDown));
    }

    #[test]
    fn test_no_crossover() {
        let mut co = CrossOver::new();
        assert_eq!(co.next(1.0, 2.0), None);
        // fast 仍在 slow 下方，无交叉
        assert_eq!(co.next(1.5, 2.0), None);
    }

    #[test]
    fn test_crossover_equal_then_cross() {
        let mut co = CrossOver::new();
        // 相等时不算交叉
        assert_eq!(co.next(2.0, 2.0), None);
        // 从相等变为 fast > slow -> CrossUp（因为 prev_fast <= prev_slow 且 fast > slow）
        assert_eq!(co.next(3.0, 2.0), Some(CrossSignal::CrossUp));
    }

    #[test]
    fn test_crossover_reset() {
        let mut co = CrossOver::new();
        co.next(1.0, 2.0);
        co.reset();
        // 重置后第一对不产生信号
        assert_eq!(co.next(3.0, 2.0), None);
    }
}
