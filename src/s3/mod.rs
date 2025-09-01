//! 静态文件服务器的 S3 操作模块。
//!
//! 该模块处理与 AWS S3 的所有交互，包括检查键是否存在和检索对象。

use crate::config::{get_bucket_name, get_s3_client};
use anyhow::Result;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

/// 检查 S3 存储桶中是否存在指定键。
///
/// # 参数
///
/// * `key` - 要检查的 S3 键。
///
/// # 返回值
///
/// 如果键存在则返回 `true`，否则返回 `false`。
pub async fn check_key_exists(key: &str) -> bool {
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name().await;

    let result = s3_client
        .head_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;

    result.is_ok()
}

/// 查找请求文件的 S3 键。
///
/// 此函数实现了 SPA 支持的回退逻辑：
/// 1. 如果请求的是目录（以 '/' 结尾）或空路径，检查该目录下的 index.html。
/// 2. 检查请求的文件是否存在（非目录路径）。
/// 3. 如果请求的是文件，检查同名的 .html 文件。
/// 4. 检查第一级目录中的 index.html。
/// 5. 检查根目录中的 index.html。
///
/// # 参数
///
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
pub async fn find_exists_key(pathname: &str) -> Option<String> {
    // 1. 如果请求的是目录（以 '/' 结尾）或空路径，检查该目录下的 index.html
    if pathname.is_empty() || pathname.ends_with('/') {
        let index_path = if pathname.is_empty() {
            "index.html".to_string()
        } else {
            format!("{}index.html", pathname)
        };
        if check_key_exists(&index_path).await {
            return Some(index_path);
        }
    }

    // 2. 检查请求的文件是否存在（非目录路径）
    if !pathname.is_empty() && !pathname.ends_with('/') && check_key_exists(pathname).await {
        return Some(pathname.to_string());
    }

    // 3. 如果请求的是文件，检查同名的 .html 文件
    if !pathname.is_empty() && !pathname.ends_with('/') {
        let html_path = if pathname.ends_with(".html") {
            pathname.to_string()
        } else {
            format!("{}.html", pathname)
        };
        if check_key_exists(&html_path).await {
            return Some(html_path);
        }
    }

    // 4. 检查第一级目录中的 index.html
    if !pathname.is_empty() && !pathname.ends_with('/') {
        // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
        let first_level_dir = pathname.split('/').next().unwrap_or("");

        if !first_level_dir.is_empty() {
            let first_level_index = format!("{}/index.html", first_level_dir);
            if check_key_exists(&first_level_index).await {
                return Some(first_level_index);
            }
        }
    }

    // 5. 检查根目录中的 index.html
    let root_index = "index.html";
    if check_key_exists(root_index).await {
        return Some(root_index.to_string());
    }

    None
}

/// 生成 S3 对象的预签名 URL（过期时间为1小时）。
///
/// # 参数
///
/// * `key` - S3 对象的键。
///
/// # 返回值
///
/// 预签名 URL 或错误信息。
pub async fn generate_presigned_url(key: &str) -> Result<String> {
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name().await;

    // 构建预签名配置（1小时过期时间）
    let presigning_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(3600)) // 1小时 = 3600秒
        .build()?;

    // 生成预签名请求
    let presigned_request = s3_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .presigned(presigning_config)
        .await?;

    Ok(presigned_request.uri().to_string())
}
