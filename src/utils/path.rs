use std::path::Path;

/// 默认的索引文件名
const INDEX_FILE: &str = "index.html";

/// 从文件路径或文件名中获取扩展名，并转换为小写
///
/// # 参数
///
/// * `path` - 文件路径或文件名
///
/// # 返回值
///
/// 返回小写的文件扩展名字符串，如果没有扩展名则返回空字符串
///
/// # 示例
///
/// ```
/// use static_server::utils::path::get_extension_lowercase;
///
/// assert_eq!(get_extension_lowercase("file.TXT"), "txt");
/// assert_eq!(get_extension_lowercase("path/to/image.PNG"), "png");
/// assert_eq!(get_extension_lowercase("noext"), "");
/// assert_eq!(get_extension_lowercase(".hidden"), "");
/// ```
pub fn get_extension_lowercase(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default()
}

/// 规范化请求路径
///
/// 移除前导和尾随斜杠，检查空路径
///
/// # 参数
/// * `path` - 原始请求路径
///
/// # 返回值
/// * `Some(String)` - 规范化后的路径
/// * `None` - 路径为空或仅包含空白字符
///
/// # 示例
/// ```
/// use static_server::utils::path::normalize_path;
///
/// assert_eq!(normalize_path("/api/v1/"), Some("api/v1".to_string()));
/// assert_eq!(normalize_path("api/v1"), Some("api/v1".to_string()));
/// assert_eq!(normalize_path("/"), None);
/// assert_eq!(normalize_path(""), None);
/// assert_eq!(normalize_path("   "), None);
/// ```
pub fn normalize_path(path: &str) -> Option<String> {
    let path = path.trim_start_matches('/').trim_end_matches('/');
    if path.is_empty() || path.trim().is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// 生成 SPA fallback 路径列表
///
/// 根据请求路径生成需要检查的 index.html 路径列表。
/// 从当前目录开始，逐级向上回退到根目录。
///
/// # 参数
/// * `pathname` - 规范化后的请求路径（不含前导/尾随斜杠）
/// * `prefix` - S3 键前缀（如 "www"）
///
/// # 返回值
/// 需要检查的 S3 键列表，按优先级排序
///
/// # 示例
/// ```
/// use static_server::utils::path::generate_fallback_paths;
///
/// let paths = generate_fallback_paths("app/page", "www");
/// assert_eq!(paths, vec![
///     "www/app/page/index.html",
///     "www/app/index.html",
///     "www/index.html",
/// ]);
/// ```
pub fn generate_fallback_paths(pathname: &str, prefix: &str) -> Vec<String> {
    let mut paths = Vec::new();

    // 1. 首先检查当前目录下的 index.html
    paths.push(format!("{}/{}/{}", prefix, pathname, INDEX_FILE));

    // 2. 逐级向上回退
    let parts: Vec<&str> = pathname.split('/').collect();
    for i in (1..parts.len()).rev() {
        let parent_path = parts[..i].join("/");
        paths.push(format!("{}/{}/{}", prefix, parent_path, INDEX_FILE));
    }

    // 3. 最后检查根目录
    paths.push(format!("{}/{}", prefix, INDEX_FILE));

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_extension_lowercase() {
        assert_eq!(get_extension_lowercase("file.TXT"), "txt");
        assert_eq!(get_extension_lowercase("path/to/image.PNG"), "png");
        assert_eq!(get_extension_lowercase("noext"), "");
        assert_eq!(get_extension_lowercase(".hidden"), "");
        assert_eq!(get_extension_lowercase("file.tar.gz"), "gz");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/api/v1/"), Some("api/v1".to_string()));
        assert_eq!(normalize_path("api/v1"), Some("api/v1".to_string()));
        assert_eq!(normalize_path("/"), None);
        assert_eq!(normalize_path(""), None);
        assert_eq!(normalize_path("   "), None);
    }

    #[test]
    fn test_generate_fallback_paths() {
        let paths = generate_fallback_paths("app/page", "www");
        assert_eq!(
            paths,
            vec![
                "www/app/page/index.html",
                "www/app/index.html",
                "www/index.html",
            ]
        );

        let paths = generate_fallback_paths("deep/nested/path", "www");
        assert_eq!(
            paths,
            vec![
                "www/deep/nested/path/index.html",
                "www/deep/nested/index.html",
                "www/deep/index.html",
                "www/index.html",
            ]
        );

        let paths = generate_fallback_paths("single", "www");
        assert_eq!(paths, vec!["www/single/index.html", "www/index.html",]);
    }
}
