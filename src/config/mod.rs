//! 静态文件服务器的配置模块。
//!
//! 该模块负责从环境变量加载和管理配置。

use aws_config::BehaviorVersion;
use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use std::env;
use std::sync::Arc;
use tokio::sync::OnceCell;
use std::time::Duration;
use aws_sdk_s3::presigning::PresigningConfig;

// 全局单例 S3 客户端
static S3_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

// 全局存储桶名称
static BUCKET_NAME: OnceCell<String> = OnceCell::const_new();

/// 使用环境变量创建 S3 客户端。
///
/// # 返回值
///
/// 配置好的 `aws_sdk_s3::Client`。
async fn create_s3_client_inner() -> Client {
    // 从环境变量读取配置
    let s3_access_key_id = env::var("S3_ACCESS_KEY_ID").expect("必须设置 S3_ACCESS_KEY_ID");
    let s3_secret_access_key =
        env::var("S3_SECRET_ACCESS_KEY").expect("必须设置 S3_SECRET_ACCESS_KEY");
    let s3_region = env::var("S3_REGION").expect("必须设置 S3_REGION");
    let s3_endpoint = env::var("S3_ENDPOINT").expect("必须设置 S3_ENDPOINT");

    let credentials = Credentials::new(
        s3_access_key_id,
        s3_secret_access_key,
        None,
        None,
        "manual-credentials",
    );

    let region_provider = RegionProviderChain::first_try(Some(Region::new(s3_region)));

    let config_builder = aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(region_provider)
        .endpoint_url(s3_endpoint);

    let aws_config = config_builder.load().await;
    Client::new(&aws_config)
}

/// 获取全局单例 S3 客户端。
///
/// # 返回值
///
/// 全局单例 `aws_sdk_s3::Client` 的引用。
pub async fn get_s3_client() -> &'static Arc<Client> {
    S3_CLIENT
        .get_or_init(|| async { Arc::new(create_s3_client_inner().await) })
        .await
}

/// 获取全局存储桶名称。
///
/// # 返回值
///
/// 全局存储桶名称的引用。
pub async fn get_bucket_name() -> &'static str {
    BUCKET_NAME
        .get_or_init(|| async {
            env::var("S3_BUCKET").expect("必须设置 S3_BUCKET")
        })
        .await
}

/// 生成 S3 对象的预签名 URL。
///
/// # 参数
///
/// * `key` - S3 对象的键。
/// * `expires_in` - URL 过期时间（秒）。
///
/// # 返回值
///
/// 预签名 URL 或错误信息。
pub async fn generate_presigned_url(
    key: &str,
    expires_in: u64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name().await;
    
    let presigned_request = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .presigned(
            PresigningConfig::builder()
                .expires_in(Duration::from_secs(expires_in))
                .build()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
        )
        .await?;

    Ok(presigned_request.uri().to_string())
}