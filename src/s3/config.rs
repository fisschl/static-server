//! S3配置模块
//!
//! 该模块负责S3客户端的配置和初始化。

use aws_sdk_s3::Client;
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// 全局 S3 客户端实例
static S3_CLIENT: OnceCell<Arc<Client>> = OnceCell::new();

/// 异步初始化 S3 客户端
///
/// # 环境变量要求
///
/// 必需：
/// - `AWS_ACCESS_KEY_ID` - AWS 访问密钥 ID
/// - `AWS_SECRET_ACCESS_KEY` - AWS 秘密访问密钥  
/// - `AWS_REGION` - AWS 区域，如 `us-east-1`、`cn-north-1`
/// - `S3_BUCKET` - S3 存储桶名称
///
/// 可选：
/// - `AWS_ENDPOINT_URL` - S3 兼容服务端点（MinIO、阿里云 OSS 等）
///
/// # 示例配置
///
/// 阿里云 OSS：
/// ```bash
/// export AWS_ACCESS_KEY_ID=your-access-key-id
/// export AWS_SECRET_ACCESS_KEY=your-access-key-secret
/// export AWS_REGION=cn-hangzhou
/// export AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
/// export S3_BUCKET=my-bucket-name
/// ```
///
/// MinIO：
/// ```bash
/// export AWS_ACCESS_KEY_ID=minioadmin
/// export AWS_SECRET_ACCESS_KEY=minioadmin
/// export AWS_REGION=us-east-1
/// export AWS_ENDPOINT_URL=http://localhost:9000
/// export S3_BUCKET=my-bucket
/// ```
async fn init_s3_client() -> Arc<Client> {
    let config = aws_config::load_from_env().await;
    Arc::new(Client::new(&config))
}

/// 获取全局 S3 客户端实例
///
/// 如果尚未初始化，会在首次调用时异步初始化
pub async fn get_s3_client() -> Arc<Client> {
    if let Some(client) = S3_CLIENT.get() {
        return client.clone();
    }
    
    let client = init_s3_client().await;
    S3_CLIENT.set(client.clone()).ok();
    client
}

/// 全局 S3 存储桶名称缓存
static BUCKET_NAME: OnceCell<String> = OnceCell::new();

/// 获取全局 S3 存储桶名称
///
/// 首次调用时从环境变量读取并缓存，后续调用直接返回缓存值
pub fn get_bucket_name() -> String {
    BUCKET_NAME
        .get_or_init(|| {
            std::env::var("S3_BUCKET")
                .expect("S3_BUCKET must be set")
        })
        .clone()
}
