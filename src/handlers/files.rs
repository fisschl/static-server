use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use reqwest::Client;

use crate::s3::find_exists_key_with_cache;
use crate::s3::generate_presigned_url;

/// 不应缓存的文件扩展名。
const NO_CACHE_EXTS: &[&str] = &[".html", ".htm"];

/// 需要保留的响应头部列表
const PRESERVE_HEADERS: &[header::HeaderName] = &[
    header::ACCEPT_RANGES,
    header::CACHE_CONTROL,
    header::CONTENT_ENCODING,
    header::CONTENT_LANGUAGE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
    header::CONTENT_TYPE,
    header::ETAG,
    header::EXPIRES,
    header::LAST_MODIFIED,
    header::VARY,
];

/// 用于代理的请求头部列表
const FORWARD_HEADERS: &[header::HeaderName] = &[
    header::ACCEPT,
    header::ACCEPT_ENCODING,
    header::RANGE,
    header::IF_MATCH,
    header::IF_NONE_MATCH,
    header::IF_MODIFIED_SINCE,
    header::IF_UNMODIFIED_SINCE,
    header::USER_AGENT,
];

/// 确定文件扩展名是否应该被缓存。
///
/// # 参数
///
/// * `ext` - 要检查的文件扩展名。
///
/// # 返回值
///
/// 如果文件应该被缓存则返回 `true`，否则返回 `false`。
fn should_cache(ext: &str) -> bool {
    !NO_CACHE_EXTS.contains(&ext)
}

/// 处理文件请求并为静态内容提供服务。
///
/// 此函数尝试在 S3 存储桶中查找请求的文件。如果未找到文件，
/// 它会实现回退机制来为 SPA 支持提供 `index.html`。
///
/// # 参数
///
/// * `req` - HTTP 请求。
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
pub async fn handle_files(req: Request) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let pathname = if path.is_empty() { "" } else { path };

    // 查找文件
    let file_key = match find_exists_key_with_cache(pathname).await {
        Some(key) => key,
        None => return (StatusCode::NOT_FOUND, "").into_response(),
    };

    // 获取文件扩展名
    let file_ext = match std::path::Path::new(&file_key).extension() {
        Some(ext) => ext.to_str().unwrap_or(""),
        None => "",
    };

    // 生成预签名 URL
    let presigned_url = match generate_presigned_url(&file_key).await {
        Ok(url) => url,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // 使用 reqwest 客户端转发请求
    let client = Client::new();

    // 构建转发请求并复制必要的头部
    let mut forwarded_req = client.get(&presigned_url);
    for header_name in FORWARD_HEADERS {
        if let Some(value) = req.headers().get(header_name) {
            forwarded_req = forwarded_req.header(header_name, value);
        }
    }

    // 发送请求并获取响应
    let response = match forwarded_req.send().await {
        Ok(resp) => resp,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // 构建返回的响应
    let mut resp_builder = Response::builder().status(response.status());

    // 复制必要的响应头部
    for (name, value) in response.headers() {
        if PRESERVE_HEADERS.contains(name) {
            resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
        }
    }

    // 添加缓存控制头部
    if should_cache(file_ext) {
        resp_builder = resp_builder.header("cache-control", "public, max-age=2592000");
    }

    // 流式传输响应体
    match resp_builder.body(Body::from_stream(response.bytes_stream())) {
        Ok(resp) => resp,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
