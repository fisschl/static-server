//! S3操作模块
//!
//! 该模块负责处理与S3存储桶的交互操作。

use crate::s3::config::{get_bucket_name, get_s3_client};

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
    // 执行实际的 S3 检查
    let s3_client = get_s3_client();
    let bucket_name = get_bucket_name();

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
/// - 如果请求的是目录（以 '/' 结尾）或空路径，检查该目录下的 index.html。
/// - 检查请求的文件是否存在（非目录路径）。
/// - 如果请求的是文件，检查同名的 .html 文件。
/// - 检查第一级目录中的 index.html。
/// - 检查根目录中的 index.html。
///
/// # 参数
///
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
pub async fn find_exists_key(pathname: &str) -> Option<String> {
    // 1. 处理空路径情况
    if pathname.is_empty() {
        let index_path = "index.html".to_string();
        if check_key_exists(&index_path).await {
            return Some(index_path);
        }
        return None;
    }

    // 2. 处理目录路径（以 '/' 结尾）
    if pathname.ends_with('/') {
        let index_path = format!("{}index.html", pathname);
        if check_key_exists(&index_path).await {
            return Some(index_path);
        }
        return None;
    }

    // 3. 对于非空且非目录路径，检查请求的文件是否存在
    if check_key_exists(pathname).await {
        return Some(pathname.to_string());
    }

    // 4. 检查同名的 .html 文件
    let html_path = if pathname.ends_with(".html") {
        pathname.to_string()
    } else {
        format!("{}.html", pathname)
    };
    if check_key_exists(&html_path).await {
        return Some(html_path);
    }

    // 5. 检查第一级目录中的 index.html
    // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
    let first_level_dir = pathname.split('/').next().unwrap_or("");
    if !first_level_dir.is_empty() {
        let first_level_index = format!("{}/index.html", first_level_dir);
        if check_key_exists(&first_level_index).await {
            return Some(first_level_index);
        }
    }

    // 6. 检查根目录中的 index.html
    let root_index = "index.html";
    if check_key_exists(root_index).await {
        return Some(root_index.to_string());
    }

    None
}
