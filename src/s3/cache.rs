//! S3缓存模块
//!
//! 该模块负责处理S3键查找的缓存功能。

use crate::s3::s3_ops;
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
    let result = s3_ops::find_exists_key(pathname).await;

    // 将结果存入缓存并返回
    PATH_EXISTS_CACHE.insert(path_str, result.clone()).await;
    result
}
