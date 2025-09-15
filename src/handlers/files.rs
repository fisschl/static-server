use super::spa_key;
use crate::utils::s3::{generate_presigned_url, get_bucket_name};
use aws_sdk_s3::Client as S3Client;
use axum::{
    body::Body,
    extract::{Extension, Request},
    http::{HeaderValue, Response, StatusCode, header},
    response::{IntoResponse, Redirect},
};
use reqwest::Client;
use std::sync::Arc;

/// 不应缓存的文件扩展名。
const NO_CACHE_EXTS: &[&str] = &["html", "htm"];

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
];

/// 缓存控制头部值
const CACHE_CONTROL_VALUE: &str = "public, max-age=2592000";

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
    let ext = match std::path::Path::new(key).extension() {
        Some(ext) => ext.to_str().unwrap_or(""),
        None => "",
    };

    // 转换为小写进行比较
    !NO_CACHE_EXTS.contains(&ext.to_lowercase().as_str())
}

/// 处理文件请求并为静态内容提供服务。
///
/// 此函数尝试在 S3 存储桶中查找请求的文件。如果未找到文件，
/// 它会实现回退机制来为 SPA 支持提供 `index.html`。
///
/// # 参数
///
/// * `req` - HTTP 请求。
/// * `Extension(s3_client)` - S3 客户端实例。
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
pub async fn handle_files(
    Extension(s3_client): Extension<Arc<S3Client>>,
    req: Request,
) -> impl IntoResponse {
    let path = req
        .uri()
        .path()
        .trim_start_matches('/')
        .trim_end_matches('/');

    // 防御 pathname 为空的情况，若为空则重定向到 https://ys.mihoyo.com/
    if path.is_empty() {
        return Redirect::to("https://ys.mihoyo.com/").into_response();
    }

    // 生成预签名 URL
    let presigned_url =
        match generate_presigned_url(s3_client.clone(), &get_bucket_name(), path).await {
            Ok(url) => url,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
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
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // 如果响应状态码不是 404，直接返回响应
    if response.status() != StatusCode::NOT_FOUND {
        // 构建返回的响应
        let mut resp_builder = Response::builder().status(response.status());

        // 复制必要的响应头部
        for (name, value) in response.headers() {
            if PRESERVE_HEADERS.contains(name) {
                resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
            }
        }

        // 在每个分支中分别写入响应头
        if should_cache(path) {
            resp_builder = resp_builder.header(
                header::CACHE_CONTROL,
                HeaderValue::from_static(CACHE_CONTROL_VALUE),
            );
        }

        // 流式传输响应体
        match resp_builder.body(Body::from_stream(response.bytes_stream())) {
            Ok(resp) => resp,
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        // 如果响应是 404，则走 find_exists_key 逻辑（现在已经有缓存了）
        let bucket_name = get_bucket_name();
        let file_key = match spa_key::find_exists_key(s3_client.clone(), &bucket_name, path).await {
            Some(key) => key,
            None => return (StatusCode::NOT_FOUND, "File not found").into_response(),
        };

        // 重新生成预签名 URL
        let presigned_url =
            match generate_presigned_url(s3_client.clone(), &get_bucket_name(), &file_key).await {
                Ok(url) => url,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            };

        // 重新发送请求
        let mut forwarded_req = client.get(&presigned_url);
        for header_name in FORWARD_HEADERS {
            if let Some(value) = req.headers().get(header_name) {
                forwarded_req = forwarded_req.header(header_name, value);
            }
        }

        let response = match forwarded_req.send().await {
            Ok(resp) => resp,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        };

        // 构建返回的响应
        let mut resp_builder = Response::builder().status(response.status());

        // 复制必要的响应头部
        for (name, value) in response.headers() {
            if PRESERVE_HEADERS.contains(name) {
                resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
            }
        }

        // 在每个分支中分别写入响应头
        if should_cache(&file_key) {
            resp_builder = resp_builder.header(
                header::CACHE_CONTROL,
                HeaderValue::from_static(CACHE_CONTROL_VALUE),
            );
        }

        // 流式传输响应体
        match resp_builder.body(Body::from_stream(response.bytes_stream())) {
            Ok(resp) => resp,
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }
}
