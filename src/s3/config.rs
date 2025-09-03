//! S3配置模块
//!
//! 该模块负责S3客户端的配置和初始化。

use aws_sdk_s3::Client;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// 在模块加载时初始化环境变量
static ENV_INITIALIZED: OnceCell<()> = OnceCell::new();

/// 初始化环境变量（仅执行一次）
///
/// # 环境变量要求
///
/// ## 必需环境变量：
/// - `AWS_ACCESS_KEY_ID` - AWS访问密钥ID
/// - `AWS_SECRET_ACCESS_KEY` - AWS秘密访问密钥  
/// - `AWS_REGION` - AWS区域，如 `us-east-1`、`cn-north-1`
/// - `S3_BUCKET` - S3存储桶名称（必填）
///
/// ## 可选环境变量：
/// - `AWS_ENDPOINT_URL` - S3兼容服务端点（阿里云OSS等）
///
/// # 示例配置
///
/// ## 阿里云OSS：
/// ```bash
/// export AWS_ACCESS_KEY_ID=your-access-key-id
/// export AWS_SECRET_ACCESS_KEY=your-access-key-secret
/// export AWS_REGION=cn-hangzhou
/// export AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
/// export S3_BUCKET=my-bucket-name
/// ```
fn init_env() {
    ENV_INITIALIZED.get_or_init(|| {
        dotenv::dotenv().ok(); // 加载 .env 文件
    });
}

/// 全局 S3 客户端实例
static S3_CLIENT: OnceCell<Arc<Client>> = OnceCell::new();

/// 异步初始化 S3 客户端
async fn init_s3_client() -> Arc<Client> {
    init_env(); // 确保环境变量已初始化
    let config = aws_config::load_from_env().await;
    Arc::new(Client::new(&config))
}

/// 获取全局 S3 客户端实例
///
/// # 注意
/// 首次调用时会初始化S3客户端，需要确保以下环境变量已正确设置：
/// - `AWS_ACCESS_KEY_ID`
/// - `AWS_SECRET_ACCESS_KEY`
/// - `AWS_REGION`
/// - `S3_BUCKET`（必填）
/// - `AWS_ENDPOINT_URL`（可选）
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
/// # 注意
/// 需要确保 `S3_BUCKET` 环境变量已正确设置，否则会panic
///
/// # Panics
/// 如果 `S3_BUCKET` 环境变量未设置，此函数会panic
pub fn get_bucket_name() -> String {
    init_env(); // 确保环境变量已初始化
    BUCKET_NAME
        .get_or_init(|| {
            std::env::var("S3_BUCKET").expect(
                "S3_BUCKET environment variable must be set. Please set S3_BUCKET=your-bucket-name",
            )
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 测试获取存储桶名称的缓存功能
    ///
    /// 验证多次调用get_bucket_name()函数时，返回的是同一个缓存值，
    /// 确保存储桶名称只从环境变量读取一次并缓存。
    fn test_get_bucket_name_returns_cached_value() {
        let bucket1 = get_bucket_name();
        let bucket2 = get_bucket_name();
        assert_eq!(bucket1, bucket2);
    }
}
