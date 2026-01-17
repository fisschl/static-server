use crate::utils::headers::filter_headers_blacklist;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, AGE,
    CACHE_CONTROL, CONNECTION, COOKIE, EXPIRES, HOST, ORIGIN, PRAGMA, PROXY_AUTHORIZATION,
    REFERER, SET_COOKIE, TE, TRAILER, TRANSFER_ENCODING, UPGRADE, VARY,
};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, StatusCode},
    response::Response,
};

/// 请求头黑名单（需要在代理转发时移除的头）
///
/// 这些头字段在代理转发时应该被移除，因为：
/// - HOST, CONNECTION: HTTP 连接相关的头，由代理服务器重新设置
/// - TE, TRAILER, TRANSFER_ENCODING: 传输编码相关的头，由代理服务器处理
/// - UPGRADE: 协议升级头，代理服务器不支持
/// - ORIGIN, REFERER: 来源信息，可能会被 API 用于验证，应保持原值或根据需要处理
/// - PROXY_AUTHORIZATION: 代理认证，不应转发
/// - COOKIE: Cookie，由代理服务器管理
pub const REQUEST_HEADERS_BLOCKLIST: &[HeaderName] = &[
    HOST,                  // 主机名，由代理服务器设置为实际目标地址
    CONNECTION,            // 连接控制，由代理服务器管理
    TE,                    // 传输编码，由代理服务器处理
    TRAILER,               // 尾部字段，由代理服务器处理
    TRANSFER_ENCODING,      // 传输编码，由代理服务器处理
    UPGRADE,               // 协议升级，代理服务器不支持
    ORIGIN,                // 来源，可能包含敏感信息
    REFERER,               // 来源页面，可能包含敏感信息
    PROXY_AUTHORIZATION,   // 代理认证，不应转发
    COOKIE,                // Cookie，由代理服务器管理
];

/// 响应头黑名单（需要在代理转发时移除的头）
///
/// 这些头字段在返回给客户端时应该被移除，因为：
/// - CONNECTON, TE, TRAILER, TRANSFER_ENCODING, UPGRADE: HTTP 连接/传输相关头，由代理服务器处理
/// - ACCESS_CONTROL_*: CORS 相关头，应由代理服务器的 CORS 层统一管理，避免冲突
/// - VARY, SET_COOKIE, CACHE_CONTROL, EXPIRES, AGE: 缓存和 Cookie 相关头，可能会影响客户端行为
pub const RESPONSE_HEADERS_BLOCKLIST: &[HeaderName] = &[
    CONNECTION,                       // 连接控制，由代理服务器管理
    TE,                               // 传输编码，由代理服务器处理
    TRAILER,                          // 尾部字段，由代理服务器处理
    TRANSFER_ENCODING,                // 传输编码，由代理服务器处理
    UPGRADE,                          // 协议升级，代理服务器不支持
    ACCESS_CONTROL_ALLOW_ORIGIN,      // CORS 头，由代理服务器的 CorsLayer 管理
    ACCESS_CONTROL_ALLOW_METHODS,     // CORS 头，由代理服务器的 CorsLayer 管理
    ACCESS_CONTROL_ALLOW_HEADERS,     // CORS 头，由代理服务器的 CorsLayer 管理
    ACCESS_CONTROL_ALLOW_CREDENTIALS, // CORS 头，由代理服务器的 CorsLayer 管理
    ACCESS_CONTROL_EXPOSE_HEADERS,    // CORS 头，由代理服务器的 CorsLayer 管理
    ACCESS_CONTROL_MAX_AGE,           // CORS 头，由代理服务器的 CorsLayer 管理
    VARY,                             // 内容协商，可能会影响缓存
    SET_COOKIE,                       // Cookie 设置，应由代理服务器管理
    CACHE_CONTROL,                    // 缓存控制，应由代理服务器管理
    EXPIRES,                          // 过期时间，应由代理服务器管理
    AGE,                              // 缓存年龄，应由代理服务器管理
    PRAGMA,                           // HTTP/1.0 缓存控制
];

/// 通用代理请求函数
///
/// 将客户端的请求代理转发到目标 API，自动处理请求头过滤、响应头过滤等逻辑。
/// 支持流式传输请求体和响应体，适用于需要高性能代理的场景。
///
/// # 参数
///
/// * `client` - reqwest HTTP 客户端引用，用于发送 HTTP 请求
/// * `target_url` - 目标 API 的完整 URL（如 "https://api.deepseek.com/models"）
/// * `method` - HTTP 请求方法（GET, POST, PUT, DELETE 等）
/// * `headers` - 客户端传入的原始请求头，会被过滤后转发（应用层应提前处理认证）
/// * `query` - 可选的查询参数字符串（如 "model=gpt-4"），会附加到目标 URL
/// * `body` - 可选的请求体内容，支持流式传输（用于大文件或流式 API）
///
/// # 认证处理
///
/// **应用层责任**：此函数不再自动添加 Authorization 头。
/// 应用层应在调用此函数之前，根据业务逻辑判断：
/// - 如果客户端提供了 AUTHORIZATION 头，则保留
/// - 如果没有，则添加服务器配置的认证（如 Bearer Token）
///
/// # 返回值
///
/// * `Ok(Response)` - 代理成功的响应，包含过滤后的响应头和流式响应体
/// * `Err((StatusCode, String))` - 代理失败，包含 HTTP 状态码和错误描述
///
/// # 错误处理
///
/// - `INTERNAL_SERVER_ERROR (500)`: 响应构建失败时
/// - `BAD_GATEWAY (502)`: 当无法连接到目标 API 时
///
/// # 功能特性
///
/// 1. **请求头过滤**: 移除不需要转发的请求头（如 HOST, CONNECTION 等）
/// 2. **查询参数处理**: 支持将查询参数附加到目标 URL
/// 3. **流式传输**: 支持流式传输请求体和响应体，降低内存占用
/// 4. **响应头过滤**: 移除不应转发的响应头（如 CORS 相关头）
///
/// # 示例
///
/// ```rust,ignore
/// // 简单 GET 请求代理
/// let response = proxy_request(
///     &state.http_client,
///     "https://api.deepseek.com/models",
///     Method::GET,
///     headers,
///     None,  // 无查询参数
///     None,  // 无请求体
/// ).await?;
///
/// // 带 JSON 请求体的 POST 请求
/// let body = reqwest::Body::from(json_str);
/// let response = proxy_request(
///     &state.http_client,
///     "https://api.deepseek.com/chat/completions",
///     Method::POST,
///     headers,
///     Some("stream=true".to_string()),  // 查询参数
///     Some(body),                         // JSON 请求体
/// ).await?;
/// ```
pub async fn proxy_request(
    client: &reqwest::Client,
    target_url: &str,
    method: reqwest::Method,
    headers: HeaderMap,
    query: Option<String>,
    body: Option<reqwest::Body>,
) -> Result<Response, (StatusCode, String)> {
    // 1. 过滤请求头：使用统一的黑名单过滤
    let request_headers = filter_headers_blacklist(&headers, REQUEST_HEADERS_BLOCKLIST);

    // 2. 构建目标 URL：将查询参数附加到目标 URL
    let final_url = if let Some(q) = query {
        format!("{}?{}", target_url, q)
    } else {
        target_url.to_string()
    };

    // 3. 构建 HTTP 请求
    let mut request_builder = client.request(method, &final_url).headers(request_headers);

    // 4. 设置请求体（如果提供）
    if let Some(body_content) = body {
        request_builder = request_builder.body(body_content);
    }

    // 5. 发送请求到目标 API
    let response = request_builder
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 6. 获取响应状态码和过滤响应头
    let status = response.status();
    let filtered_response_headers = filter_headers_blacklist(response.headers(), RESPONSE_HEADERS_BLOCKLIST);

    // 7. 构建响应并应用过滤后的响应头
    let mut builder = Response::builder().status(status);
    for (name, value) in filtered_response_headers.iter() {
        builder = builder.header(name, value);
    }

    // 8. 流式传输响应体：将响应体转换为流，避免一次性加载到内存
    let stream = response.bytes_stream();
    let body = Body::from_stream(stream);

    // 9. 返回构建的响应
    builder
        .body(body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
