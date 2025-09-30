use http::HeaderMap;

/// 从源头部映射克隆指定的头部到新的头部映射
///
/// # 参数
///
/// * `source` - 源头部映射
/// * `allowed_headers` - 允许克隆的头部名称列表
///
/// # 返回值
///
/// 包含指定头部的新头部映射
pub fn clone_headers(source: &HeaderMap, allowed_headers: &[http::HeaderName]) -> HeaderMap {
    let mut result = HeaderMap::new();

    for header_name in allowed_headers {
        if let Some(value) = source.get(header_name) {
            result.insert(header_name.clone(), value.clone());
        }
    }

    result
}
