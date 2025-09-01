//! S3预签名URL模块
//!
//! 该模块负责生成S3对象的预签名URL。

use crate::s3::config;
use anyhow::Result;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

/// 为 S3 键生成预签名 URL。
///
/// # 参数
///
/// * `key` - 要为其生成预签名 URL 的 S3 键。
///
/// # 返回值
///
/// 预签名 URL 的字符串表示。
pub async fn generate_presigned_url(key: &str) -> Result<String> {
    let s3_client = config::get_s3_client().await;
    let bucket_name = config::get_bucket_name();

    // 创建预签名配置，设置 URL 1 小时后过期
    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

    // 生成预签名 URL
    let presigned_request = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .presigned(presigning_config)
        .await?;

    Ok(presigned_request.uri().to_string())
}
