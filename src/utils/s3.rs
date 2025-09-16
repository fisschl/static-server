use anyhow::Result;
use aws_sdk_s3::{Client, presigning::PresigningConfig};
use cached::proc_macro::cached;
use std::sync::Arc;
use std::time::Duration;

/// 获取全局 S3 存储桶名称
///
/// # 注意
/// 需要确保 `S3_BUCKET` 环境变量已正确设置，否则会panic
///
/// # Panics
/// 如果 `S3_BUCKET` 环境变量未设置，此函数会panic
#[cached(
    time = 600  // 10分钟过期
)]
pub fn get_bucket_name() -> String {
    std::env::var("S3_BUCKET")
        .expect("S3_BUCKET environment variable must be set. Please set S3_BUCKET=your-bucket-name")
}

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
) -> Result<String> {
    // 创建预签名配置，设置 URL 1 小时后过期
    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

    // 生成预签名 URL
    let presigned_request = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(object)
        .presigned(presigning_config)
        .await?;

    Ok(presigned_request.uri().to_string())
}
