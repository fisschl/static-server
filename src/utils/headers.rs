use http::HeaderMap;
use mime_guess::MimeGuess;

/// 过滤头部映射，移除黑名单中的头部（黑名单模式）
///
/// 此函数遍历源头部映射，将不在黑名单中的所有头部复制到新的头部映射中。
/// 适用于需要保留大部分头部，仅排除特定头部的场景（如移除跨域相关头部）。
///
/// # 参数
///
/// * `source` - 源头部映射，包含原始响应的所有头部
/// * `blocked_headers` - 需要排除的头部名称列表（黑名单）
///
/// # 返回值
///
/// 返回一个新的 `HeaderMap`，包含所有不在黑名单中的头部。
pub fn filter_headers_blacklist(
    source: &HeaderMap,
    blocked_headers: &[http::HeaderName],
) -> HeaderMap {
    let mut result = HeaderMap::new();

    // 遍历源头部映射中的所有头部
    for (name, value) in source.iter() {
        // 如果头部不在黑名单中，则保留
        if !blocked_headers.contains(name) {
            result.insert(name.clone(), value.clone());
        }
    }

    result
}

/// 根据文件路径猜测 MIME 类型
///
/// # 参数
///
/// * `path` - 文件路径或文件名
///
/// # 返回值
///
/// 返回猜测的 MIME 类型字符串，如果无法猜测则返回 None
pub fn guess_mime_type(path: &str) -> Option<String> {
    let mime_guess = MimeGuess::from_path(path);
    mime_guess.first().map(|mime| mime.to_string())
}
