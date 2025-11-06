use std::path::Path;

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
