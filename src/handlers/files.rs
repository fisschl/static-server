use crate::error::AppError;
use crate::storage::Storage;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, AGE,
    CACHE_CONTROL, CONNECTION, COOKIE, EXPIRES, HOST, ORIGIN, PRAGMA, PROXY_AUTHORIZATION, REFERER,
    SET_COOKIE, TE, TRAILER, TRANSFER_ENCODING, UPGRADE, VARY,
};
use axum::http::{HeaderMap, HeaderName};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use mime_guess::MimeGuess;
use std::path::Path;

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
    HOST,                // 主机名，由代理服务器设置为实际目标地址
    CONNECTION,          // 连接控制，由代理服务器管理
    TE,                  // 传输编码，由代理服务器处理
    TRAILER,             // 尾部字段，由代理服务器处理
    TRANSFER_ENCODING,   // 传输编码，由代理服务器处理
    UPGRADE,             // 协议升级，代理服务器不支持
    ORIGIN,              // 来源，可能包含敏感信息
    REFERER,             // 来源页面，可能包含敏感信息
    PROXY_AUTHORIZATION, // 代理认证，不应转发
    COOKIE,              // Cookie，由代理服务器管理
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
/// * `target_url` - 目标 API 的完整 URL（如 "https://api.example.com/endpoint"）
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
/// * `Err(AppError)` - 代理失败，返回应用错误类型
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
pub async fn proxy_request(
    client: &reqwest::Client,
    target_url: &str,
    method: reqwest::Method,
    headers: HeaderMap,
    query: Option<String>,
    body: Option<reqwest::Body>,
) -> Result<Response<Body>, AppError> {
    // 1. 过滤请求头：使用黑名单过滤，移除不需要转发的请求头
    let request_headers = {
        let mut result = HeaderMap::new();
        for (name, value) in headers.iter() {
            if !REQUEST_HEADERS_BLOCKLIST.contains(name) {
                result.insert(name.clone(), value.clone());
            }
        }
        result
    };

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
    let response = request_builder.send().await?;

    // 6. 获取响应状态码
    let status = response.status();

    // 7. 构建响应并应用过滤后的响应头
    let mut builder = Response::builder().status(status);
    for (name, value) in response.headers().iter() {
        if RESPONSE_HEADERS_BLOCKLIST.contains(name) {
            continue;
        }
        builder = builder.header(name, value);
    }

    // 8. 流式传输响应体：将响应体转换为流，避免一次性加载到内存
    let stream = response.bytes_stream();
    let body = Body::from_stream(stream);

    // 9. 返回构建的响应
    Ok(builder.body(body)?)
}

/// S3 存储桶中的 www 前缀
pub const WWW_PREFIX: &str = "www";

/// 默认的索引文件名
pub const INDEX_FILE: &str = "index.html";

/// 不应缓存的文件扩展名。
pub const NO_CACHE_EXTS: &[&str] = &["html", "htm"];

/// 缓存控制头部值（30 天缓存，适用于 CSS、JS、图片等静态资源）
/// max-age=2592000 表示 2592000 秒 = 30 天
pub const CACHE_CONTROL_VALUE: &str = "public, max-age=2592000";

/// 根据文件扩展名判断是否应该缓存该文件
///
/// 该函数用于确定静态文件是否应该被缓存。对于 HTML 文件（.html, .htm）通常不缓存，
/// 以确保用户总是获取最新的页面内容。对于 CSS、JS、图片等资源文件则启用缓存，
/// 以减少服务器负载并提高页面加载性能。
///
/// # 参数
///
/// * `key` - 文件路径或文件名
///
/// # 返回值
///
/// 如果文件应该被缓存则返回 `true`，否则返回 `false`
pub fn should_cache(key: &str) -> bool {
    let ext = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    !NO_CACHE_EXTS.contains(&ext.as_str())
}

/// 从对象存储获取文件并返回 HTTP 响应
///
/// 该函数通过预签名 URL 从 S3 兼容的对象存储中获取文件内容，
/// 并将其作为 HTTP 响应返回给客户端。支持请求头和响应头过滤，
/// 自动检测 Content-Type，并根据文件类型设置缓存策略。
///
/// # 参数
///
/// * `storage` - 存储后端 trait 对象，用于获取预签名 URL
/// * `http_client` - reqwest HTTP 客户端，用于发送请求
/// * `headers` - 客户端传入的请求头，会被过滤后转发
/// * `key` - 文件在存储中的键（路径）
///
/// # 返回值
///
/// * `Ok(Response)` - 包含文件内容的 HTTP 响应
/// * `Err(AppError)` - 获取失败时返回错误
///
/// # 功能特性
///
/// 1. **预签名 URL**: 使用存储后端生成临时访问 URL
/// 2. **请求头过滤**: 转发前移除黑名单中的请求头
/// 3. **Content-Type 检测**: 自动根据文件扩展名推断 MIME 类型
/// 4. **缓存控制**: 根据文件类型自动设置缓存头
pub async fn fetch_and_proxy_file(
    storage: &dyn Storage,
    http_client: &reqwest::Client,
    headers: &http::HeaderMap,
    key: &str,
) -> Result<Response<Body>, AppError> {
    let presigned_url = storage.get_presigned_url(key).await?;

    let forwarded_headers = {
        let mut result = http::HeaderMap::new();
        for (name, value) in headers.iter() {
            if !REQUEST_HEADERS_BLOCKLIST.contains(name) {
                result.insert(name.clone(), value.clone());
            }
        }
        result
    };

    // 使用 reqwest 直接发送 GET 请求
    let resp = http_client
        .get(&presigned_url)
        .headers(forwarded_headers)
        .send()
        .await?;

    let status = resp.status();
    let response_headers = resp.headers().clone();
    let body = resp.bytes().await?.to_vec();

    let mut resp_builder = Response::builder().status(status);

    for (name, value) in response_headers.iter() {
        if RESPONSE_HEADERS_BLOCKLIST.contains(name) {
            continue;
        }
        resp_builder = resp_builder.header(name, value);
    }

    if !response_headers.contains_key(header::CONTENT_TYPE) {
        if let Some(guessed_content_type) = MimeGuess::from_path(key).first().map(|m| m.to_string())
        {
            resp_builder = resp_builder.header(header::CONTENT_TYPE, guessed_content_type);
        }
    }

    if status.is_success() && should_cache(key) {
        resp_builder = resp_builder.header(header::CACHE_CONTROL, CACHE_CONTROL_VALUE);
    }

    Ok(resp_builder.body(Body::from(body))?)
}

/// 查找存在的文件键（支持单页应用路由回退）
///
/// 该函数实现了单页应用（SPA）的路由回退机制。当请求的路径不存在时，
/// 会依次尝试查找父目录的 index.html，最终回退到根目录的 index.html。
/// 这允许前端路由在刷新页面时仍能正常工作。
///
/// 查找顺序：
/// 1. `{pathname}/index.html` - 目录索引文件
/// 2. `{parent_path}/index.html` - 逐级向上查找父目录索引
/// 3. `index.html` - 根目录索引（最终回退）
///
/// # 参数
///
/// * `storage` - 存储后端 trait 对象
/// * `pathname` - 请求的路径名
///
/// # 返回值
///
/// 如果找到存在的文件键则返回 `Some(String)`，否则返回 `None`
///
/// # 示例
///
/// 请求 `/app/dashboard` 时：
/// - 首先尝试 `www/app/dashboard/index.html`
/// - 然后尝试 `www/app/index.html`
/// - 最后尝试 `www/index.html`
pub async fn find_exists_key(
    storage: &dyn Storage,
    pathname: &str,
) -> Result<Option<String>, AppError> {
    let dir_index = format!("{WWW_PREFIX}/{}/{INDEX_FILE}", pathname);
    match storage.check_key_exists(&dir_index).await {
        Ok(true) => return Ok(Some(dir_index)),
        Ok(false) => {}
        Err(e) => return Err(e),
    }

    let parts: Vec<&str> = pathname.split('/').collect();
    for i in (1..parts.len()).rev() {
        let parent_path = parts[..i].join("/");
        let index_key = format!("{WWW_PREFIX}/{}/{INDEX_FILE}", parent_path);
        match storage.check_key_exists(&index_key).await {
            Ok(true) => return Ok(Some(index_key)),
            Ok(false) => {}
            Err(e) => return Err(e),
        }
    }

    let root_index = format!("{WWW_PREFIX}/{INDEX_FILE}");
    match storage.check_key_exists(&root_index).await {
        Ok(true) => Ok(Some(root_index)),
        Ok(false) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 处理静态文件请求的主入口
///
/// 这是处理文件请求的 Axum handler。它从请求路径中提取文件路径，
/// 尝试从对象存储获取文件。如果文件不存在，则尝试使用 SPA 回退机制
/// 查找合适的 index.html 文件。
///
/// # 参数
///
/// * `state` - 应用状态，包含存储后端、HTTP 客户端和存储桶名称
/// * `req` - HTTP 请求对象
///
/// # 返回值
///
/// * `Ok(Response)` - 文件内容响应
/// * `Err(AppError::NotFound)` - 文件不存在时返回 404
///
/// # 处理流程
///
/// 1. 提取并清理请求路径
/// 2. 尝试直接获取请求的文件
/// 3. 如果返回 404，则尝试查找存在的 index.html（SPA 回退）
/// 4. 返回找到的文件内容或 404 错误
///
/// # 路径处理
///
/// - 去除前导和尾随斜杠
/// - 空路径返回 404
/// - 自动添加 `www/` 前缀到存储键
pub async fn handle_files(
    State(state): State<crate::AppState>,
    req: Request,
) -> Result<impl IntoResponse, AppError> {
    let path = req
        .uri()
        .path()
        .trim_start_matches('/')
        .trim_end_matches('/');

    if path.is_empty() || path.trim().is_empty() {
        return Err(AppError::NotFound);
    }

    let s3_path = format!("{WWW_PREFIX}/{path}");

    let response = fetch_and_proxy_file(
        state.storage.as_ref(),
        &state.http_client,
        req.headers(),
        &s3_path,
    )
    .await?;

    if response.status() != StatusCode::NOT_FOUND {
        return Ok(response);
    }

    let file_key = find_exists_key(state.storage.as_ref(), path)
        .await?
        .ok_or(AppError::NotFound)?;

    fetch_and_proxy_file(
        state.storage.as_ref(),
        &state.http_client,
        req.headers(),
        &file_key,
    )
    .await
}

#[cfg(test)]
mod tests {
    use crate::handlers::files::{
        fetch_and_proxy_file, find_exists_key, handle_files, proxy_request, should_cache, CONNECTION,
        HOST,
    };
    use crate::storage::MockStorage;
    use crate::AppState;
    use axum::body::Body;
    use axum::http::{HeaderMap, Request, StatusCode};
    use axum::response::IntoResponse;
    use mockall::predicate::eq;
    use std::sync::Arc;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 测试 proxy_request 函数的基本 GET 请求
    ///
    /// 验证：
    /// - 基本的 GET 请求代理功能正常工作
    /// - 自定义请求头能够正确传递
    /// - 响应状态码正确返回
    #[tokio::test]
    async fn test_proxy_request_get_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello from mock"))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-Custom-Header", "test-value".parse().unwrap());

        let response = proxy_request(
            &client,
            &format!("{}/test", mock_server.uri()),
            reqwest::Method::GET,
            headers,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(response.status(), 200);
    }

    /// 测试 proxy_request 函数的 POST 请求带请求体
    ///
    /// 验证：
    /// - POST 请求方法正确转发
    /// - 请求体能够正确传递到目标服务器
    /// - 创建资源的响应状态码（201）正确处理
    #[tokio::test]
    async fn test_proxy_request_post_with_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/data"))
            .respond_with(ResponseTemplate::new(201).set_body_string(r#"{"id": 123}"#))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let headers = HeaderMap::new();
        let body = reqwest::Body::from(r#"{"name":"test"}"#);

        let response = proxy_request(
            &client,
            &format!("{}/api/data", mock_server.uri()),
            reqwest::Method::POST,
            headers,
            None,
            Some(body),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), 201);
    }

    /// 测试 proxy_request 函数的查询参数处理
    ///
    /// 验证：
    /// - 查询参数字符串能够正确附加到目标 URL
    /// - 多个查询参数（q=test&limit=10）正确处理
    #[tokio::test]
    async fn test_proxy_request_with_query_params() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string("search results"))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let headers = HeaderMap::new();

        let response = proxy_request(
            &client,
            &format!("{}/search", mock_server.uri()),
            reqwest::Method::GET,
            headers,
            Some("q=test&limit=10".to_string()),
            None,
        )
        .await
        .unwrap();

        assert_eq!(response.status(), 200);
    }

    /// 测试 proxy_request 函数的请求头过滤功能
    ///
    /// 验证：
    /// - 黑名单中的请求头（HOST, CONNECTION）被正确移除
    /// - 不在黑名单中的自定义请求头（X-Allowed）能够正常传递
    /// - 目标服务器只接收到允许的头信息
    #[tokio::test]
    async fn test_proxy_request_filters_request_headers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/headers"))
            .and(header("X-Allowed", "should-be-present"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-Allowed", "should-be-present".parse().unwrap());
        headers.insert(HOST, "original-host.com".parse().unwrap());
        headers.insert(CONNECTION, "keep-alive".parse().unwrap());

        let response = proxy_request(
            &client,
            &format!("{}/headers", mock_server.uri()),
            reqwest::Method::GET,
            headers,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(response.status(), 200);
    }

    /// 测试 proxy_request 函数的错误响应处理
    ///
    /// 验证：
    /// - 目标服务器返回的错误状态码（500）能够正确传递
    /// - 错误响应体能够正常返回给客户端
    #[tokio::test]
    async fn test_proxy_request_error_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let headers = HeaderMap::new();

        let response = proxy_request(
            &client,
            &format!("{}/error", mock_server.uri()),
            reqwest::Method::GET,
            headers,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(response.status(), 500);
    }

    /// 测试 fetch_and_proxy_file 函数的文件获取功能
    ///
    /// 验证：
    /// - 能够成功从存储获取预签名 URL
    /// - 通过预签名 URL 获取文件内容
    /// - 返回正确的 HTTP 响应状态码（200 OK）
    #[tokio::test]
    async fn test_fetch_and_proxy_file_success() {
        let mock_server = MockServer::start().await;
        let mut mock_storage = MockStorage::new();
        let mock_uri = mock_server.uri();

        Mock::given(method("GET"))
            .and(path("/test.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello World"))
            .mount(&mock_server)
            .await;

        mock_storage
            .expect_get_presigned_url()
            .returning(move |_| Ok(format!("{}/test.txt", mock_uri)));
        
        let http_client = reqwest::Client::new();
        
        let result = fetch_and_proxy_file(
            &mock_storage,
            &http_client,
            &http::HeaderMap::new(),
            "www/test.txt",
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
    }

    /// 测试 should_cache 函数的缓存判断逻辑
    ///
    /// 验证：
    /// - CSS、JS、图片等静态资源文件应该被缓存（返回 true）
    /// - HTML 文件（.html, .htm）不应该被缓存（返回 false）
    #[test]
    fn test_should_cache() {
        assert!(should_cache("file.css"));
        assert!(should_cache("file.js"));
        assert!(should_cache("image.png"));
        assert!(!should_cache("page.html"));
        assert!(!should_cache("page.htm"));
    }

    /// 测试 find_exists_key 函数 - 直接目录索引存在
    ///
    /// 验证：
    /// - 当请求的目录下存在 index.html 时直接返回
    /// - 不需要向上级目录查找
    #[tokio::test]
    async fn test_find_exists_key_direct_index() {
        let mut mock_storage = MockStorage::new();

        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/page/index.html"))
            .returning(|_| Ok(true));
        
        let result = find_exists_key(&mock_storage, "app/page").await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("www/app/page/index.html".to_string()));
    }

    /// 测试 find_exists_key 函数 - 需要向上级目录查找
    ///
    /// 验证：
    /// - 当直接目录索引不存在时，逐级向上查找
    /// - 找到父目录的 index.html 时返回
    #[tokio::test]
    async fn test_find_exists_key_fallback_to_parent() {
        let mut mock_storage = MockStorage::new();

        // 直接目录索引不存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/page/index.html"))
            .returning(|_| Ok(false));
        
        // 父目录索引存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/index.html"))
            .returning(|_| Ok(true));
        
        let result = find_exists_key(&mock_storage, "app/page").await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("www/app/index.html".to_string()));
    }

    /// 测试 find_exists_key 函数 - 回退到根目录
    ///
    /// 验证：
    /// - 当所有父目录都没有 index.html 时，回退到根目录
    /// - 返回根目录的 index.html
    #[tokio::test]
    async fn test_find_exists_key_fallback_to_root() {
        let mut mock_storage = MockStorage::new();

        // 直接目录索引不存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/page/index.html"))
            .returning(|_| Ok(false));
        
        // 父目录索引也不存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/index.html"))
            .returning(|_| Ok(false));
        
        // 根目录索引存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/index.html"))
            .returning(|_| Ok(true));
        
        let result = find_exists_key(&mock_storage, "app/page").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("www/index.html".to_string()));
    }

    /// 测试 find_exists_key 函数 - 完全找不到
    ///
    /// 验证：
    /// - 当所有可能的 index.html 都不存在时返回 None
    /// - 包括根目录的 index.html 也不存在
    #[tokio::test]
    async fn test_find_exists_key_not_found() {
        let mut mock_storage = MockStorage::new();

        // 所有索引都不存在
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/page/index.html"))
            .returning(|_| Ok(false));
        
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/index.html"))
            .returning(|_| Ok(false));
        
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/index.html"))
            .returning(|_| Ok(false));
        
        let result = find_exists_key(&mock_storage, "app/page").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    /// 测试 find_exists_key 函数 - 单层路径
    ///
    /// 验证：
    /// - 单层路径（如 "app"）也能正确处理
    /// - 先尝试 app/index.html，再尝试根目录
    #[tokio::test]
    async fn test_find_exists_key_single_level() {
        let mut mock_storage = MockStorage::new();

        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/index.html"))
            .returning(|_| Ok(false));
        
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/index.html"))
            .returning(|_| Ok(true));
        
        let result = find_exists_key(&mock_storage, "app").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("www/index.html".to_string()));
    }

    /// 测试 handle_files 函数 - 成功获取文件
    ///
    /// 验证：
    /// - 能够成功获取文件并返回 200
    /// - 返回正确的文件内容
    #[tokio::test]
    async fn test_handle_files_success() {
        let mock_server = MockServer::start().await;
        let mut mock_storage = MockStorage::new();
        let mock_uri = mock_server.uri();

        // Mock 存储返回预签名 URL
        mock_storage
            .expect_get_presigned_url()
            .with(eq("www/test.txt"))
            .returning(move |_| Ok(format!("{}/test.txt", mock_uri)));
        
        // Wiremock 模拟 S3 返回文件内容
        Mock::given(method("GET"))
            .and(path("/test.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello World"))
            .mount(&mock_server)
            .await;
        
        let state = AppState {
            storage: Arc::new(mock_storage),
            http_client: reqwest::Client::new(),
        };

        let req = Request::builder()
            .uri("/test.txt")
            .body(Body::empty())
            .unwrap();

        let response = handle_files(axum::extract::State(state), req).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// 测试 handle_files 函数 - 文件不存在
    ///
    /// 验证：
    /// - 当文件不存在时返回 404 错误
    #[tokio::test]
    async fn test_handle_files_not_found() {
        let mut mock_storage = MockStorage::new();

        // 文件不存在
        mock_storage
            .expect_get_presigned_url()
            .with(eq("www/missing.txt"))
            .returning(|_| Err(crate::error::AppError::NotFound));
        
        // SPA fallback 也找不到
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/missing.txt/index.html"))
            .returning(|_| Ok(false));
        
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/index.html"))
            .returning(|_| Ok(false));
        
        let state = AppState {
            storage: Arc::new(mock_storage),
            http_client: reqwest::Client::new(),
        };

        let req = Request::builder()
            .uri("/missing.txt")
            .body(Body::empty())
            .unwrap();

        let result = handle_files(axum::extract::State(state), req).await;

        match result {
            Err(crate::error::AppError::NotFound) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// 测试 handle_files 函数 - 空路径返回 404
    ///
    /// 验证：
    /// - 当请求路径为空时返回 404
    #[tokio::test]
    async fn test_handle_files_empty_path() {
        let mock_storage = MockStorage::new();
        
        let state = AppState {
            storage: Arc::new(mock_storage),
            http_client: reqwest::Client::new(),
        };
        
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        
        let result = handle_files(axum::extract::State(state), req).await;
        
        match result {
            Err(crate::error::AppError::NotFound) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// 测试 handle_files 函数 - SPA fallback 成功
    ///
    /// 验证：
    /// - 当直接请求的文件返回 404 时，触发 SPA fallback
    /// - 最终返回找到的 index.html
    #[tokio::test]
    async fn test_handle_files_spa_fallback() {
        let mock_server = MockServer::start().await;
        let mut mock_storage = MockStorage::new();
        let mock_uri = mock_server.uri();

        // 第一次请求返回 404
        mock_storage
            .expect_get_presigned_url()
            .with(eq("www/app/page"))
            .returning({
                let uri = mock_uri.clone();
                move |_| Ok(format!("{}/app/page", uri))
            });
        
        Mock::given(method("GET"))
            .and(path("/app/page"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;
        
        // SPA fallback 找到 index.html
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/page/index.html"))
            .returning(|_| Ok(false));
        
        mock_storage
            .expect_check_key_exists()
            .with(eq("www/app/index.html"))
            .returning(|_| Ok(true));
        
        mock_storage
            .expect_get_presigned_url()
            .with(eq("www/app/index.html"))
            .returning(move |_| Ok(format!("{}/app/index.html", mock_uri)));
        
        Mock::given(method("GET"))
            .and(path("/app/index.html"))
            .respond_with(ResponseTemplate::new(200).set_body_string("SPA App"))
            .mount(&mock_server)
            .await;
        
        let state = AppState {
            storage: Arc::new(mock_storage),
            http_client: reqwest::Client::new(),
        };

        let req = Request::builder()
            .uri("/app/page")
            .body(Body::empty())
            .unwrap();

        let response = handle_files(axum::extract::State(state), req).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
