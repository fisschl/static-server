use super::constants::WWW_PREFIX;
use aws_sdk_s3::Client as S3Client;
use cached::proc_macro::cached;
use std::sync::Arc;
use tokio::time::Duration;

/// 检查 S3 存储桶中是否存在指定键。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `bucket_name` - S3 存储桶名称。
/// * `key` - 要检查的 S3 键。
///
/// # 返回值
///
/// 如果键存在则返回 `true`，否则返回 `false`。
pub async fn check_key_exists(s3_client: Arc<S3Client>, bucket_name: &str, key: &str) -> bool {
    // 执行实际的 S3 检查
    let result = s3_client
        .head_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;

    result.is_ok()
}

/// 默认的索引文件名
const INDEX_FILE: &str = "index.html";

/// 查找请求文件的 S3 键。
///
/// 此函数实现了 SPA 支持的回退逻辑：
/// - 检查第一级目录中的 index.html。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `bucket_name` - S3 存储桶名称。
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
#[cached(
    key = "String",
    convert = r#"{ format!("{}:{}", bucket_name, pathname) }"#,
    size = 32768,
    time = 120
)]
pub async fn find_exists_key(
    s3_client: Arc<S3Client>,
    bucket_name: &str,
    pathname: &str,
) -> Option<String> {
    // 1. 检查第一级目录中的 index.html（在 www 前缀下）
    // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
    let first_level_dir = pathname.split('/').next().unwrap_or("");
    if !first_level_dir.is_empty() {
        let first_level_index = format!("{}/{}/{}", WWW_PREFIX, first_level_dir, INDEX_FILE);
        if check_key_exists(s3_client.clone(), bucket_name, &first_level_index).await {
            return Some(first_level_index);
        }
    }

    None
}
