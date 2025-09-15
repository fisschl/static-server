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

/// 使用正斜杠连接多个字符串组件
/// 
/// 这个函数类似于 Node.js 中的 path.join，但专门使用正斜杠(/)作为分隔符。
/// 它会自动处理组件前后的斜杠，确保结果中组件之间只有一个正斜杠。
/// 
/// # 参数
/// 
/// * `components` - 要连接的字符串组件切片
/// 
/// # 返回值
/// 
/// 连接后的字符串，组件之间使用单个正斜杠分隔
/// 
/// # 示例
/// 
/// ```
/// use static_server::utils::cache::join_slash;
/// 
/// assert_eq!(join_slash(&["find-exists-key", "path"]), "find-exists-key/path");
/// assert_eq!(join_slash(&["find-exists-key/", "/path"]), "find-exists-key/path");
/// assert_eq!(join_slash(&["find-exists-key", "", "path"]), "find-exists-key/path");
/// ```
pub fn join_slash(components: &[&str]) -> String {
    components
        .iter()
        .map(|s| s.trim_matches('/')) // 去掉每个组件前后的斜杠
        .filter(|s| !s.is_empty()) // 过滤掉空字符串
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_slash() {
        // 基本用法
        assert_eq!(join_slash(&["find-exists-key", "path"]), "find-exists-key/path");
        
        // 处理前后斜杠
        assert_eq!(join_slash(&["find-exists-key/", "/path"]), "find-exists-key/path");
        
        // 处理空字符串
        assert_eq!(join_slash(&["find-exists-key", "", "path"]), "find-exists-key/path");
        
        // 处理多个斜杠
        assert_eq!(join_slash(&["find-exists-key//", "//path"]), "find-exists-key/path");
        
        // 单个组件
        assert_eq!(join_slash(&["find-exists-key"]), "find-exists-key");
        
        // 空组件
        assert_eq!(join_slash(&[]), "");
        
        // 全是空字符串
        assert_eq!(join_slash(&["", "", ""]), "");
        
        // 只有斜杠的组件
        assert_eq!(join_slash(&["/", "/"]), "");
        
        // 混合情况
        assert_eq!(join_slash(&["/find-exists-key/", "", "/path/"]), "find-exists-key/path");
    }
}
