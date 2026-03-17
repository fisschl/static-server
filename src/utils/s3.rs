use crate::error::AppError;
use aws_sdk_s3::Client;
use cached::proc_macro::cached;
use std::sync::Arc;
use std::time::Duration;

/// 为 S3 键生成预签名 URL。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `bucket_name` - S3 存储桶名称。
/// * `key` - 要为其生成预签名 URL 的 S3 键。
///
/// # 返回值
///
/// 预签名 URL 的字符串表示。
///
/// # Errors
///
/// 当无法生成预签名 URL 时返回错误。
#[cached(
    key = "String",
    convert = r#"{ format!("{}:{}", bucket_name, object) }"#,
    size = 8192,    // 8 * 1024 最大容量
    time = 1800,    // 30分钟过期（30 * 60 = 1800秒）
    result = true   // 缓存Result类型
)]
pub async fn generate_presigned_url(
    s3_client: Arc<Client>,
    bucket_name: &str,
    object: &str,
) -> Result<String, AppError> {
    // 创建预签名配置，设置 URL 1 小时后过期
    let presigning_config =
        aws_sdk_s3::presigning::PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|e| AppError::S3(e.to_string()))?;

    // 生成预签名 URL
    let presigned_request = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(object)
        .presigned(presigning_config)
        .await
        .map_err(|e| AppError::S3(e.to_string()))?;

    Ok(presigned_request.uri().to_string())
}
