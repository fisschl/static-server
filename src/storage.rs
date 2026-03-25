use async_trait::async_trait;
use crate::error::AppError;

/// 键状态枚举，用于区分"不存在"和"S3 错误"
#[derive(Debug, Clone, PartialEq)]
pub enum KeyStatus {
    Exists,
    NotFound,
}

/// 存储抽象接口
///
/// 通过 trait 抽象 S3 操作，使业务逻辑可测试
#[mockall::automock]
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// 生成预签名 URL
    async fn get_presigned_url(&self, bucket: &str, key: &str) -> Result<String, AppError>;

    /// 检查对象是否存在
    /// 返回 KeyStatus，不存在时返回 NotFound（不区分 S3 错误，避免 Result 开销）
    async fn check_key_exists(&self, bucket: &str, key: &str) -> KeyStatus;
}

/// S3 存储实现
#[derive(Clone)]
pub struct S3Storage {
    client: std::sync::Arc<aws_sdk_s3::Client>,
}

impl S3Storage {
    pub fn new(client: std::sync::Arc<aws_sdk_s3::Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn get_presigned_url(&self, bucket: &str, key: &str) -> Result<String, AppError> {
        let presigning_config =
            aws_sdk_s3::presigning::PresigningConfig::expires_in(std::time::Duration::from_secs(3600))
                .map_err(|e| AppError::S3(e.to_string()))?;

        let presigned_request = self.client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        Ok(presigned_request.uri().to_string())
    }

    async fn check_key_exists(&self, bucket: &str, key: &str) -> KeyStatus {
        match self.client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => KeyStatus::Exists,
            Err(_) => KeyStatus::NotFound,
        }
    }
}
