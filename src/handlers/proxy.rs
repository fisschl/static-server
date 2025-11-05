use crate::handlers::constants::{
    CACHE_CONTROL_VALUE, FORWARD_HEADERS, NO_CACHE_EXTS, PRESERVE_HEADERS,
};
use crate::utils::headers::clone_headers;
use crate::utils::headers::guess_mime_type;
use crate::utils::s3::{generate_presigned_url, get_bucket_name};
use aws_sdk_s3::Client as S3Client;
use axum::{
    body::Body,
    http::{Response, StatusCode, header},
};
use reqwest::Client;
use std::sync::Arc;

/// 确定文件键是否应该被缓存。
///
/// # 参数
///
/// * `key` - 要检查的文件键。
///
/// # 返回值
///
/// 如果文件应该被缓存则返回 `true`，否则返回 `false`。
fn should_cache(key: &str) -> bool {
    // 获取文件扩展名
    let ext = std::path::Path::new(key)
        .extension()
        .map_or("", |ext| ext.to_str().unwrap_or(""));

    // 转换为小写进行比较
    !NO_CACHE_EXTS.contains(&ext.to_lowercase().as_str())
}

/// 从 S3 获取文件内容并返回响应
///
/// 此函数封装了生成预签名 URL、发送请求和处理响应的逻辑。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `headers` - 原始 HTTP 请求的头部。
/// * `key` - 要获取的 S3 对象键。
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
///
/// # Errors
///
/// 当无法生成预签名 URL 或发送 HTTP 请求失败时返回错误。
pub async fn fetch_and_proxy_file(
    s3_client: Arc<S3Client>,
    headers: &http::HeaderMap,
    key: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    // 生成预签名 URL
    let presigned_url =
        match generate_presigned_url(s3_client.clone(), &get_bucket_name(), key).await {
            Ok(url) => url,
            Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("S3 Error: {}", e))),
        };

    // 使用 reqwest 客户端转发请求
    let client = Client::new();

    // 构建转发请求并复制必要的头部
    let forwarded_headers = clone_headers(headers, FORWARD_HEADERS);
    let forwarded_req = client.get(&presigned_url).headers(forwarded_headers);

    // 发送请求并获取响应
    let response = match forwarded_req.send().await {
        Ok(resp) => resp,
        Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("Proxy Error: {}", e))),
    };

    // 构建返回的响应
    let mut resp_builder = Response::builder().status(response.status());

    // 复制必要的响应头部
    for header_name in PRESERVE_HEADERS {
        if let Some(value) = response.headers().get(header_name) {
            resp_builder = resp_builder.header(header_name, value);
        }
    }

    // 如果 S3 响应缺少 Content-Type，尝试猜测
    if !response.headers().contains_key(header::CONTENT_TYPE) {
        if let Some(guessed_content_type) = guess_mime_type(key) {
            resp_builder = resp_builder.header(header::CONTENT_TYPE, guessed_content_type);
        }
    }

    // 添加缓存控制头部（仅对成功响应）
    if response.status().is_success() && should_cache(key) {
        resp_builder = resp_builder.header(header::CACHE_CONTROL, CACHE_CONTROL_VALUE);
    }

    // 流式传输响应体
    match resp_builder.body(Body::from_stream(response.bytes_stream())) {
        Ok(resp) => Ok(resp),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Response Error: {}", e),
        )),
    }
}
