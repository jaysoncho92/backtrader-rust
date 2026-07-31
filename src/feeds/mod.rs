// 数据 Feed 模块

mod r#trait;
mod csv;

pub use r#trait::DataFeed;
pub use csv::CsvFeed;
