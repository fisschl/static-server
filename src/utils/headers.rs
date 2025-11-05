use http::HeaderMap;
use mime_guess::MimeGuess;

/// 从源头部映射克隆指定的头部到新的头部映射
///
/// 此函数通过遍历允许的头部列表，从源头部映射中提取匹配的头部，
/// 避免了不必要的头部名称克隆操作，提高了性能。
///
/// # 性能优化说明
///
/// - 遍历允许的头部列表（9个），而不是实际存在的头部（可能几十个）
/// - 避免了不必要的头部名称克隆操作
/// - 只对匹配的头部进行头部值克隆
/// - 减少了整体的处理时间和内存分配
///
/// # 参数
///
/// * `source` - 源头部映射，包含原始请求的所有头部
/// * `allowed_headers` - 允许克隆的头部名称列表，通常包含安全且需要转发的头部
///
/// # 返回值
///
/// 返回一个新的 `HeaderMap`，包含所有在允许列表中的头部。
/// 如果源头部映射中没有匹配的头部，则返回空的头部映射。
pub fn clone_headers(source: &HeaderMap, allowed_headers: &[http::HeaderName]) -> HeaderMap {
    let mut result = HeaderMap::new();

    // 遍历允许的头部列表，而不是源头部映射
    // 这种方法更高效，因为：
    // 1. 允许的头部数量（9个）远少于实际请求中的头部数量
    // 2. 典型HTTP请求包含大量头部（Cookie、User-Agent、各种自定义头部等）
    // 3. 只需检查少数几个头部，避免遍历所有实际存在的头部
    for header_name in allowed_headers {
        if let Some(value) = source.get(header_name) {
            // 使用引用而不是克隆头部名称，避免不必要的字符串复制
            result.insert(header_name, value.clone());
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
