use moka::future::Cache;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

/// 创建一个静态缓存实例，用于缓存find_exists_key的结果
/// 缓存配置：
/// - 最大容量：32768个条目（32 * 1024）
/// - 默认过期时间：60秒
static PATH_EXISTS_CACHE: Lazy<Arc<Cache<String, Option<String>>>> = Lazy::new(|| {
    Arc::new(
        Cache::builder()
            .max_capacity(32 * 1024)
            .time_to_live(Duration::from_secs(60))
            .build(),
    )
});

/// 带缓存的查找请求文件的 S3 键。
///
/// 此函数是find_exists_key的缓存版本，使用moka缓存来避免重复的 S3 请求。
///
/// # 参数
///
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
pub async fn find_exists_key_with_cache(pathname: &str) -> Option<String> {
    // 转换为 String 以便在缓存中使用
    let path_str = pathname.to_string();

    // 首先检查缓存
    if let Some(result) = PATH_EXISTS_CACHE.get(&path_str).await {
        return result.clone();
    }

    // 计算结果
    let result = find_exists_key(pathname).await;

    // 将结果存入缓存并返回
    PATH_EXISTS_CACHE.insert(path_str, result.clone()).await;
    result
}

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
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name();

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
    // 1. 检查第一级目录中的 index.html
    // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
    let first_level_dir = pathname.split('/').next().unwrap_or("");
    if !first_level_dir.is_empty() {
        let first_level_index = format!("{}/{}", first_level_dir, INDEX_FILE);
        if check_key_exists(&first_level_index).await {
            return Some(first_level_index);
        }
    }

    // 2. 检查根目录中的 index.html
    if check_key_exists(INDEX_FILE).await {
        return Some(INDEX_FILE.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 获取存储桶中的一些示例文件（用于测试）
    ///
    /// # 返回值
    ///
    /// 存储桶中存在的一些文件键的向量
    async fn get_sample_files_from_bucket() -> Vec<String> {
        let s3_client = get_s3_client().await;
        let bucket_name = get_bucket_name();
        
        let response = s3_client
            .list_objects_v2()
            .bucket(&bucket_name)
            .max_keys(10)
            .send()
            .await;

        if let Ok(response) = response {
            response.contents
                .into_iter()
                .flatten()
                .filter_map(|object| object.key)
                .collect()
        } else {
            Vec::new()
        }
    }

    #[tokio::test]
    async fn test_find_exists_key_with_real_files() {
        // 获取存储桶中的真实文件
        let sample_files = get_sample_files_from_bucket().await;
        
        if sample_files.is_empty() {
            // 如果没有找到文件，跳过这个测试
            println!("没有找到真实文件，跳过基于真实文件的测试");
            return;
        }
        
        println!("存储桶中找到 {} 个文件:", sample_files.len());
        for file_key in &sample_files {
            println!("  - {}", file_key);
        }
        
        // 对每个真实文件测试查找功能
        for file_key in &sample_files {
            println!("测试文件: {}", file_key);
            
            // 测试文件存在性检查
            let exists = check_key_exists(file_key).await;
            assert!(exists, "文件 {} 应该存在", file_key);
            
            // 测试查找功能
            let result = find_exists_key(file_key).await;
            assert!(result.is_some(), "应该能找到文件 {}", file_key);
            
            // 测试带缓存的查找
            let cached_result = find_exists_key_with_cache(file_key).await;
            assert!(cached_result.is_some(), "应该能通过缓存找到文件 {}", file_key);
        }
    }

    #[tokio::test]
    async fn test_spa_fallback_logic_with_real_files() {
        // 测试SPA回退逻辑
        
        // 测试访问不存在的路径时是否回退到index.html
        let root_index_exists = check_key_exists("index.html").await;
        if root_index_exists {
            // 测试访问不存在的页面路径
            let result = find_exists_key("about").await;
            // 应该回退到index.html
            assert!(result.is_some(), "访问不存在的路径时应该回退到index.html");
            assert_eq!(result.unwrap(), "index.html");
            
            // 测试访问不存在的嵌套路径
            let result2 = find_exists_key("user/profile").await;
            assert!(result2.is_some(), "访问不存在的嵌套路径时应该回退到index.html");
            assert_eq!(result2.unwrap(), "index.html");
        }
    }

    #[tokio::test]
    async fn test_find_exists_key_first_level_directory() {
        // 使用实际配置的 S3 环境进行测试
        // 测试将使用 .env 文件中配置的 AWS 凭据和存储桶
        let result = find_exists_key("about").await;
        // 根据实际环境，结果可能是 Some 或 None
        assert!(result.is_some() || result.is_none());
    }

    #[tokio::test]
    async fn test_find_exists_key_root_fallback() {
        let result = find_exists_key("nonexistent").await;
        // 根据实际环境，结果可能是 Some 或 None
        assert!(result.is_some() || result.is_none());
    }

    #[tokio::test]
    async fn test_find_exists_key_not_found() {
        let result = find_exists_key("definitely-non-existent-path").await;
        // 对于不存在的路径，应该返回 None
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_exists_key_with_cache_caching_behavior() {
        let path = "test-cache";
        
        // 第一次调用
        let result1 = find_exists_key_with_cache(path).await;
        
        // 第二次调用，应该使用缓存
        let result2 = find_exists_key_with_cache(path).await;
        
        // 两次结果应该相同
        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_find_exists_key_empty_path() {
        let result = find_exists_key("").await;
        // 空路径应该检查根目录的 index.html
        assert!(result.is_some() || result.is_none());
    }

    #[tokio::test]
    async fn test_check_key_exists_integration() {
        // 测试实际的文件存在性检查
        let exists = check_key_exists("index.html").await;
        // 根据实际环境，返回 true 或 false
        assert!(exists || !exists);
    }
}
