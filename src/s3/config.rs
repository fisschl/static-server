//! S3配置模块
//!
//! 该模块负责S3客户端的配置和初始化。

use aws_sdk_s3::Client;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局 S3 客户端实例
///
/// 使用官方标准方式从环境变量中自动读取 AWS 配置信息来创建 S3 客户端。
/// 所有配置都通过标准 AWS 环境变量自动处理。
///
/// # 标准 AWS 环境变量
///
/// * `AWS_ACCESS_KEY_ID` - AWS 访问密钥 ID
/// * `AWS_SECRET_ACCESS_KEY` - AWS 秘密访问密钥
/// * `AWS_REGION` - AWS 区域（默认：us-east-1）
/// * `AWS_ENDPOINT_URL` - S3 兼容服务的端点 URL
///
/// # Panics
///
/// 如果无法获取 tokio 运行时或初始化失败，会 panic
static S3_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    let rt = tokio::runtime::Handle::try_current().expect("No tokio runtime found");

    let config = rt.block_on(aws_config::load_from_env());
    Arc::new(Client::new(&config))
});

/// 全局 S3 存储桶名称
///
/// # Panics
///
/// 如果 S3_BUCKET 环境变量未设置，会 panic
static BUCKET_NAME: Lazy<String> =
    Lazy::new(|| std::env::var("S3_BUCKET").expect("S3_BUCKET must be set"));

/// 获取全局 S3 客户端实例。
///
/// # 返回值
///
/// 返回全局 S3 客户端实例的引用
pub fn get_s3_client() -> Arc<Client> {
    S3_CLIENT.clone()
}

/// 获取全局 S3 存储桶名称。
///
/// # 返回值
///
/// 返回全局 S3 存储桶名称的引用
pub fn get_bucket_name() -> String {
    BUCKET_NAME.clone()
}
