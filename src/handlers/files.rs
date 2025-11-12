use crate::utils::headers::filter_headers_blacklist;
use crate::utils::headers::guess_mime_type;
use crate::utils::path::get_extension_lowercase;
use crate::utils::s3::generate_presigned_url;
use crate::utils::s3::get_bucket_name;
use aws_sdk_s3::Client as S3Client;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use cached::proc_macro::cached;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

/// S3 存储桶中的 www 前缀
pub const WWW_PREFIX: &str = "www";

/// 默认的索引文件名
pub const INDEX_FILE: &str = "index.html";

/// 不应缓存的文件扩展名。
pub const NO_CACHE_EXTS: &[&str] = &["html", "htm"];

/// 需要移除的响应头部黑名单
///
/// 采用黑名单模式，移除以下头部，保留所有其他头部：
/// - 跨域相关头部（ACCESS_CONTROL_*）
/// - 缓存控制相关头部（CACHE_CONTROL, EXPIRES, PRAGMA, AGE）
pub const BLOCKED_HEADERS: &[header::HeaderName] = &[
    // 跨域相关头部
    header::ACCESS_CONTROL_ALLOW_ORIGIN,
    header::ACCESS_CONTROL_ALLOW_METHODS,
    header::ACCESS_CONTROL_ALLOW_HEADERS,
    header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
    header::ACCESS_CONTROL_EXPOSE_HEADERS,
    header::ACCESS_CONTROL_MAX_AGE,
    // 缓存控制相关头部
    header::CACHE_CONTROL,
    header::EXPIRES,
    header::PRAGMA,
    header::AGE,
];

/// 缓存控制头部值（30 天缓存，适用于 CSS、JS、图片等静态资源）
/// max-age=2592000 表示 2592000 秒 = 30 天
pub const CACHE_CONTROL_VALUE: &str = "public, max-age=2592000";

/// 请求转发时需要移除的头部黑名单
///
/// 采用黑名单模式，移除以下头部，保留所有其他头部：
/// - 连接管理相关：CONNECTION, KEEP_ALIVE, TRANSFER_ENCODING, UPGRADE
/// - 代理相关：PROXY_AUTHENTICATE, PROXY_AUTHORIZATION, PROXY_CONNECTION
/// - 主机相关：HOST（由 reqwest 自动设置）
/// - 认证相关：AUTHORIZATION, COOKIE（S3 预签名 URL 已包含认证）
/// - 源信息：ORIGIN, REFERER（避免泄露内部信息）
/// - 缓存控制：CACHE_CONTROL, PRAGMA（由程序控制）
pub const FORWARD_BLOCKED_HEADERS: &[header::HeaderName] = &[
    // 连接管理相关
    header::CONNECTION,
    header::TRANSFER_ENCODING,
    header::UPGRADE,
    // 代理相关
    header::PROXY_AUTHORIZATION,
    // 主机相关
    header::HOST,
    // 认证相关
    header::AUTHORIZATION,
    header::COOKIE,
    // 源信息
    header::ORIGIN,
    header::REFERER,
    // 缓存控制
    header::CACHE_CONTROL,
    header::PRAGMA,
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
    // 获取文件扩展名并转换为小写
    let ext = get_extension_lowercase(key);

    // 检查是否在不缓存列表中
    !NO_CACHE_EXTS.contains(&ext.as_str())
}

/// 从 S3 获取文件内容并返回响应
///
/// 此函数封装了生成预签名 URL、发送请求和处理响应的逻辑。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `http_client` - HTTP 客户端实例。
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
    http_client: Arc<Client>,
    headers: &http::HeaderMap,
    key: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    // 生成预签名 URL
    let presigned_url =
        match generate_presigned_url(s3_client.clone(), &get_bucket_name(), key).await {
            Ok(url) => url,
            Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("S3 Error: {}", e))),
        };

    // 使用黑名单模式过滤并转发请求头部
    let forwarded_headers = filter_headers_blacklist(headers, FORWARD_BLOCKED_HEADERS);
    let forwarded_req = http_client.get(&presigned_url).headers(forwarded_headers);

    // 发送请求并获取响应
    let response = match forwarded_req.send().await {
        Ok(resp) => resp,
        Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("Proxy Error: {}", e))),
    };

    // 构建返回的响应
    let mut resp_builder = Response::builder().status(response.status());

    // 使用黑名单模式复制响应头部（移除跨域相关头部，保留其他所有头部）
    let filtered_headers = filter_headers_blacklist(response.headers(), BLOCKED_HEADERS);
    for (name, value) in filtered_headers.iter() {
        resp_builder = resp_builder.header(name, value);
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

/// 检查 S3 存储桶中是否存在指定键。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `bucket_name` - S3 存储桶名称。
/// * `key` - 要检查的 S3 键。
///
/// # 返回值
///
/// 如果键存在则返回 `true`，否则返回 `false`。
pub async fn check_key_exists(s3_client: Arc<S3Client>, bucket_name: &str, key: &str) -> bool {
    // 执行实际的 S3 检查
    let result = s3_client
        .head_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;

    result.is_ok()
}

/// 查找请求文件的 S3 键。
///
/// 此函数实现了 SPA 支持的回退逻辑：
/// - 检查第一级目录中的 index.html。
///
/// # 参数
///
/// * `s3_client` - S3 客户端实例。
/// * `bucket_name` - S3 存储桶名称。
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
#[cached(
    key = "String",
    convert = r#"{ format!("{}:{}", bucket_name, pathname) }"#,
    size = 32768,
    time = 120
)]
pub async fn find_exists_key(
    s3_client: Arc<S3Client>,
    bucket_name: &str,
    pathname: &str,
) -> Option<String> {
    // 1. 检查第一级目录中的 index.html（在 www 前缀下）
    // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
    let first_level_dir = pathname.split('/').next().unwrap_or("");
    if !first_level_dir.is_empty() {
        let first_level_index = format!("{WWW_PREFIX}/{first_level_dir}/{INDEX_FILE}");
        if check_key_exists(s3_client.clone(), bucket_name, &first_level_index).await {
            return Some(first_level_index);
        }
    }

    None
}

/// 处理文件请求并为静态内容提供服务。
///
/// 此函数尝试在 S3 存储桶中查找请求的文件。如果未找到文件，
/// 它会实现回退机制来为 SPA 支持提供 `index.html`。
///
/// # 参数
///
/// * `State(state)` - 应用状态，包含 S3 和 HTTP 客户端。
/// * `req` - HTTP 请求。
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
pub async fn handle_files(State(state): State<crate::AppState>, req: Request) -> impl IntoResponse {
    let path = req
        .uri()
        .path()
        .trim_start_matches('/')
        .trim_end_matches('/');

    // 在 /www 前缀下查找文件
    let s3_path = format!("{WWW_PREFIX}/{path}");

    // 尝试直接获取请求的文件
    match fetch_and_proxy_file(
        state.s3_client.clone(),
        state.http_client.clone(),
        req.headers(),
        &s3_path,
    )
    .await
    {
        // 如果成功获取文件且不是 404，直接返回响应
        Ok(response) if response.status() != StatusCode::NOT_FOUND => {
            return response.into_response();
        }
        // 如果是 404，继续下面的回退逻辑
        Ok(_) => {}
        // 如果出现错误，直接返回错误响应
        Err((status, msg)) => return (status, msg).into_response(),
    }

    // 如果响应是 404，则走 find_exists_key 逻辑（现在已经有缓存了）
    let bucket_name = get_bucket_name();
    let Some(file_key) = find_exists_key(state.s3_client.clone(), &bucket_name, path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // 使用 fetch_and_proxy_file 获取回退文件
    match fetch_and_proxy_file(state.s3_client, state.http_client, req.headers(), &file_key).await {
        Ok(response) => response.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
