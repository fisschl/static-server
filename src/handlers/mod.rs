//! 静态文件服务器的 HTTP 处理模块。
//!
//! 该模块包含用于从 S3 提供文件服务的主要请求处理函数。

use crate::s3::{find_exists_key, generate_presigned_url};
use actix_web::{HttpRequest, HttpResponse, Result};
use awc::Client;

/// 不应缓存的文件扩展名。
const NO_CACHE_EXTS: &[&str] = &[".html", ".htm"];

/// 需要保留的响应头部列表
const PRESERVE_HEADERS: &[&str] = &[
    "accept-ranges",
    "cache-control",
    "content-encoding",
    "content-language",
    "content-length",
    "content-range",
    "content-type",
    "etag",
    "expires",
    "last-modified",
    "vary",
];

/// 用于代理的请求头部列表
const FORWARD_HEADERS: &[&str] = &[
    "accept",
    "accept-encoding",
    "range",
    "if-match",
    "if-none-match",
    "if-modified-since",
    "if-unmodified-since",
    "user-agent",
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
pub async fn serve_files(req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    let path = req.path().trim_start_matches('/');
    let pathname = if path.is_empty() { "" } else { path };

    // 查找文件
    let file_key = match find_exists_key(pathname).await {
        Some(key) => key,
        None => return Ok(HttpResponse::NotFound().body("Not Found")),
    };

    // 获取文件扩展名
    let file_ext = match std::path::Path::new(&file_key).extension() {
        Some(ext) => ext.to_str().unwrap_or(""),
        None => "",
    };

    // 生成预签名 URL
    let presigned_url = generate_presigned_url(&file_key)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 使用 awc 客户端转发请求
    let client = Client::default();

    // 构建转发请求并复制必要的头部
    let mut forwarded_req = client.get(&presigned_url);
    for header_name in FORWARD_HEADERS {
        if let Some(value) = req.headers().get(*header_name) {
            forwarded_req = forwarded_req.insert_header((*header_name, value.as_bytes()));
        }
    }

    // 发送请求并获取响应
    let response = forwarded_req
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 构建返回的响应
    let mut resp = HttpResponse::build(response.status());

    // 明确列出需要保留的响应头部，确保只转发必要的信息
    // 这种白名单方式比黑名单方式更安全，避免意外暴露后端信息

    // 复制必要的响应头部
    for (name, value) in response.headers() {
        let name_str = name.as_str().to_lowercase();
        if PRESERVE_HEADERS.contains(&name_str.as_str()) {
            resp.insert_header((name.as_str(), value.as_bytes()));
        }
    }

    // 添加缓存控制头部
    if should_cache(file_ext) {
        resp.insert_header(("cache-control", "public, max-age=2592000"));
    }

    // 流式传输响应体
    Ok(resp.streaming(response))
}
