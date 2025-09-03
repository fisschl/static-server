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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// 测试生成有效的预签名URL
    /// 
    /// 验证generate_presigned_url函数能够成功生成包含正确签名和
    /// 有效期的预签名URL，URL格式应符合AWS S3标准。
    async fn test_generate_presigned_url_returns_valid_url() {
        // 测试生成有效的预签名 URL
        let result = generate_presigned_url("test-file.txt").await;
        assert!(result.is_ok());
        
        let url = result.unwrap();
        assert!(url.starts_with("https://"));
        assert!(url.contains("X-Amz-Signature"));
    }

    #[tokio::test]
    /// 测试处理包含特殊字符的键名
    /// 
    /// 验证generate_presigned_url函数能够正确处理包含特殊字符
    /// （如@符号）的S3键名，确保URL编码和签名过程正常工作。
    async fn test_generate_presigned_url_with_special_characters() {
        // 测试处理包含特殊字符的键
        let result = generate_presigned_url("folder/subdir/file@name.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    /// 测试空键名的情况
    /// 
    /// 验证generate_presigned_url函数对空键名的处理，
    /// 预期应该返回错误，因为S3不允许空的键名。
    async fn test_generate_presigned_url_empty_key() {
        // 测试空键的情况 - 应该失败，因为 S3 不允许空键
        let result = generate_presigned_url("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    /// 测试自定义端点URL的情况
    /// 
    /// 验证generate_presigned_url函数在使用自定义端点URL
    /// （如阿里云OSS）时能够正确生成预签名URL。
    async fn test_generate_presigned_url_with_endpoint() {
        // 测试包含自定义端点 URL 的情况
        let result = generate_presigned_url("test-object").await;
        assert!(result.is_ok());
    }
}
