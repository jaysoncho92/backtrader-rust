use chrono::NaiveDateTime;
use std::path::Path;

use crate::core::Bar;
use super::DataFeed;

/// CsvFeed: 从 CSV 文件加载 OHLCV 数据到内存
/// 支持配置列名映射和日期格式
pub struct CsvFeed {
    bars: Vec<Bar>,
    cursor: usize,
}

impl CsvFeed {
    /// 从 CSV 文件创建 CsvFeed，使用默认列名映射
    /// 默认列名: Date,Open,High,Low,Close,Volume
    /// 默认日期格式: %Y-%m-%d
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, CsvFeedError> {
        Self::with_options(path, "%Y-%m-%d")
    }

    /// 指定日期格式加载 CSV
    pub fn with_options<P: AsRef<Path>>(path: P, date_format: &str) -> Result<Self, CsvFeedError> {
        let mut rdr = csv::Reader::from_path(path.as_ref())
            .map_err(|e| CsvFeedError::IoError(e.to_string()))?;

        let headers = rdr
            .headers()
            .map_err(|e| CsvFeedError::ParseError(e.to_string()))?
            .clone();

        // 自动检测列索引
        let date_idx = find_header(&headers, &["Date", "date", "datetime", "Datetime", "timestamp"])?;
        let open_idx = find_header(&headers, &["Open", "open"])?;
        let high_idx = find_header(&headers, &["High", "high"])?;
        let low_idx = find_header(&headers, &["Low", "low"])?;
        let close_idx = find_header(&headers, &["Close", "close"])?;
        let vol_idx = find_header(&headers, &["Volume", "volume", "Vol", "vol"]).ok();

        let mut bars = Vec::new();
        for result in rdr.records() {
            let record = result.map_err(|e| CsvFeedError::ParseError(e.to_string()))?;

            let date_str = record
                .get(date_idx)
                .ok_or_else(|| CsvFeedError::ParseError("缺少日期列".to_string()))?;

            // 尝试多种日期格式解析
            let datetime = parse_datetime(date_str, date_format)?;

            let open: f64 = record.get(open_idx)
                .ok_or_else(|| CsvFeedError::ParseError("缺少 Open 列".to_string()))?
                .parse()
                .map_err(|e| CsvFeedError::ParseError(format!("Open 解析失败: {}", e)))?;

            let high: f64 = record.get(high_idx)
                .ok_or_else(|| CsvFeedError::ParseError("缺少 High 列".to_string()))?
                .parse()
                .map_err(|e| CsvFeedError::ParseError(format!("High 解析失败: {}", e)))?;

            let low: f64 = record.get(low_idx)
                .ok_or_else(|| CsvFeedError::ParseError("缺少 Low 列".to_string()))?
                .parse()
                .map_err(|e| CsvFeedError::ParseError(format!("Low 解析失败: {}", e)))?;

            let close: f64 = record.get(close_idx)
                .ok_or_else(|| CsvFeedError::ParseError("缺少 Close 列".to_string()))?
                .parse()
                .map_err(|e| CsvFeedError::ParseError(format!("Close 解析失败: {}", e)))?;

            let volume: f64 = vol_idx
                .and_then(|idx| record.get(idx))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            bars.push(Bar::new(datetime, open, high, low, close, volume, 0.0));
        }

        Ok(Self { bars, cursor: 0 })
    }
}

impl DataFeed for CsvFeed {
    fn next_bar(&mut self) -> Option<Bar> {
        if self.cursor < self.bars.len() {
            let bar = self.bars[self.cursor].clone();
            self.cursor += 1;
            Some(bar)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }

    fn len(&self) -> usize {
        self.bars.len()
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

/// CSV Feed 错误类型
#[derive(Debug, thiserror::Error)]
pub enum CsvFeedError {
    #[error("IO 错误: {0}")]
    IoError(String),
    #[error("解析错误: {0}")]
    ParseError(String),
    #[error("缺少列: {0}")]
    MissingColumn(String),
}

/// 在 CSV headers 中查找匹配的列索引
fn find_header(headers: &csv::StringRecord, candidates: &[&str]) -> Result<usize, CsvFeedError> {
    for (idx, header) in headers.iter().enumerate() {
        for &candidate in candidates {
            if header.trim() == candidate {
                return Ok(idx);
            }
        }
    }
    Err(CsvFeedError::MissingColumn(format!(
        "未找到列: {:?}",
        candidates
    )))
}

/// 尝试多种格式解析日期时间
fn parse_datetime(date_str: &str, format: &str) -> Result<NaiveDateTime, CsvFeedError> {
    let s = date_str.trim();

    // 先尝试带时间的格式
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }

    // 尝试用户指定的日期格式（只有日期，时间默认为 00:00:00）
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, format) {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }

    // 兜底: %Y-%m-%d
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }

    Err(CsvFeedError::ParseError(format!(
        "无法解析日期: '{}'",
        s
    )))
}
