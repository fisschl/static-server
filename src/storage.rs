use crate::error::AppError;
use async_trait::async_trait;
use aws_sdk_s3::error::ProvideErrorMetadata;

/// 存储抽象接口
///
/// 通过 trait 抽象 S3 操作，使业务逻辑可测试
#[mockall::automock]
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// 生成预签名 URL
    async fn get_presigned_url(&self, key: &str) -> Result<String, AppError>;

    /// 检查对象是否存在
    /// 返回 KeyStatus，不存在时返回 NotFound（不区分 S3 错误，避免 Result 开销）
    async fn check_key_exists(&self, key: &str) -> Result<bool, AppError>;
}

/// S3 存储实现
#[derive(Clone)]
pub struct S3Storage {
    client: std::sync::Arc<aws_sdk_s3::Client>,
    bucket_name: String,
}

impl S3Storage {
    pub fn new(client: std::sync::Arc<aws_sdk_s3::Client>, bucket_name: String) -> Self {
        Self { client, bucket_name }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn get_presigned_url(&self, key: &str) -> Result<String, AppError> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(3600),
        )
        .map_err(|e| AppError::S3(e.to_string()))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        Ok(presigned_request.uri().to_string())
    }

    async fn check_key_exists(&self, key: &str) -> Result<bool, AppError> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await;

        let err = match result {
            Ok(_) => return Ok(true),
            Err(e) => e,
        };

        let Some(head_error) = err.as_service_error() else {
            return Err(AppError::S3(format!("Request failed: {}", err)));
        };

        if head_error.is_not_found() {
            return Ok(false);
        }

        let err = match head_error.code() {
            Some("AccessDenied") => AppError::S3("Access denied".to_string()),
            Some("NoSuchBucket") => AppError::S3(format!("Bucket '{}' not found", self.bucket_name)),
            _ => AppError::S3(format!(
                "S3 error: {} - {}",
                head_error.code().unwrap_or("Unknown"),
                head_error.message().unwrap_or("No message")
            )),
        };

        Err(err)
    }
}
