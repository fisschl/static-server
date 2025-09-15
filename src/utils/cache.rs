use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// 创建短时缓存实例
///
/// 缓存配置：
/// - 最大容量：32768个条目（32 * 1024）
/// - 默认过期时间：120秒
pub fn create_short_cache() -> Arc<Cache<String, Option<String>>> {
    Arc::new(
        Cache::builder()
            .max_capacity(32 * 1024)
            .time_to_live(Duration::from_secs(120))
            .build(),
    )
}
