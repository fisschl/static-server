//! S3模块
//!
//! 该模块负责处理与S3存储桶的交互，包括配置、缓存、操作和预签名URL生成。

pub mod config;
pub mod presign;

// 重新导出常用的函数
pub use presign::generate_presigned_url;
