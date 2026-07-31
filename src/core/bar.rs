use chrono::NaiveDateTime;

/// Bar 结构：表示一根 K 线（OHLCV 数据）
#[derive(Debug, Clone)]
pub struct Bar {
    pub datetime: NaiveDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub openinterest: f64,
}

impl Bar {
    pub fn new(
        datetime: NaiveDateTime,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        openinterest: f64,
    ) -> Self {
        Self {
            datetime,
            open,
            high,
            low,
            close,
            volume,
            openinterest,
        }
    }
}
