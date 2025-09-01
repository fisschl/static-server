//! S3配置模块
//!
//! 该模块负责S3客户端的配置和初始化。

use anyhow::Result;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Region;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局 S3 客户端实例
static S3_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| Arc::new(create_s3_client_inner().unwrap()));

/// 全局 S3 存储桶名称
static BUCKET_NAME: Lazy<String> =
    Lazy::new(|| std::env::var("S3_BUCKET").expect("S3_BUCKET must be set"));

/// 创建 S3 客户端的内部实现。
///
/// 从环境变量中读取 AWS 凭证和配置信息来创建 S3 客户端。
///
/// # 环境变量
///
/// * `S3_ACCESS_KEY` - AWS 访问密钥 ID
/// * `S3_SECRET_KEY` - AWS 秘密访问密钥
/// * `S3_REGION` - AWS 区域
/// * `S3_ENDPOINT` - S3 兼容服务的端点 URL
/// * `S3_BUCKET` - S3 存储桶名称
///
/// # 返回值
///
/// 返回创建的 S3 客户端。
fn create_s3_client_inner() -> Result<Client> {
    let access_key = std::env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
    let secret_key = std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
    let region = std::env::var("S3_REGION").expect("S3_REGION must be set");
    let endpoint = std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");

    let credentials =
        aws_sdk_s3::config::Credentials::new(access_key, secret_key, None, None, "static-server");
    let config = aws_sdk_s3::config::Builder::new()
        .region(Region::new(region))
        .endpoint_url(endpoint)
        .credentials_provider(credentials)
        .build();

    Ok(Client::from_conf(config))
}

/// 获取全局 S3 客户端实例。
///
/// # 返回值
///
/// 返回全局 S3 客户端实例的引用。
pub fn get_s3_client() -> Arc<Client> {
    S3_CLIENT.clone()
}

/// 获取全局 S3 存储桶名称。
///
/// # 返回值
///
/// 返回全局 S3 存储桶名称的引用。
pub fn get_bucket_name() -> String {
    BUCKET_NAME.clone()
}
