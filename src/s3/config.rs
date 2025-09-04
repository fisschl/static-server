//! S3配置模块
//!
//! 该模块负责S3客户端相关的配置

/// 获取全局 S3 存储桶名称
///
/// # 注意
/// 需要确保 `S3_BUCKET` 环境变量已正确设置，否则会panic
///
/// # Panics
/// 如果 `S3_BUCKET` 环境变量未设置，此函数会panic
pub fn get_bucket_name() -> String {
    std::env::var("S3_BUCKET")
        .expect("S3_BUCKET environment variable must be set. Please set S3_BUCKET=your-bucket-name")
}
