use super::spa_key;
use crate::s3::generate_presigned_url;
use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Response, StatusCode},
    response::{IntoResponse, Redirect},
};
use reqwest::Client;

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
    header::VARY,
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
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
pub async fn handle_files(req: Request) -> impl IntoResponse {
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
    let presigned_url = match generate_presigned_url(path).await {
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
        // 如果响应是 404，则走 find_exists_key_with_cache 逻辑
        let file_key = match spa_key::find_exists_key_with_cache(path).await {
            Some(key) => key,
            None => return (StatusCode::NOT_FOUND, "File not found").into_response(),
        };

        // 重新生成预签名 URL
        let presigned_url = match generate_presigned_url(&file_key).await {
            Ok(url) => url,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_should_cache_html_files() {
        // 测试 HTML 文件不应缓存
        assert!(!should_cache("index.html"));
        assert!(!should_cache("about.htm"));
        assert!(!should_cache("page.html"));
        assert!(!should_cache("subdir/index.html"));
    }

    #[test]
    fn test_should_cache_static_files() {
        // 测试静态资源文件应该缓存
        assert!(should_cache("style.css"));
        assert!(should_cache("script.js"));
        assert!(should_cache("image.png"));
        assert!(should_cache("font.woff2"));
        assert!(should_cache("data.json"));
        assert!(should_cache("video.mp4"));
    }

    #[test]
    fn test_should_cache_files_without_extension() {
        // 测试无扩展名的文件应该缓存
        assert!(should_cache("file"));
        assert!(should_cache("folder/file"));
        assert!(should_cache("path-with-dashes"));
    }

    #[test]
    fn test_should_cache_case_insensitive() {
        // 测试大小写不敏感
        assert!(!should_cache("INDEX.HTML"));
        assert!(!should_cache("About.HTM"));
        assert!(should_cache("Style.CSS"));
        assert!(should_cache("Script.JS"));
    }

    #[test]
    fn test_preserve_headers_contains_expected_headers() {
        // 测试 PRESERVE_HEADERS 包含预期的头部
        let expected_headers = vec![
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

        for expected_header in expected_headers {
            assert!(PRESERVE_HEADERS.contains(&expected_header));
        }
    }

    #[test]
    fn test_forward_headers_contains_expected_headers() {
        // 测试 FORWARD_HEADERS 包含预期的头部
        let expected_headers = vec![
            header::ACCEPT,
            header::ACCEPT_ENCODING,
            header::RANGE,
            header::IF_MATCH,
            header::IF_NONE_MATCH,
            header::IF_MODIFIED_SINCE,
            header::IF_UNMODIFIED_SINCE,
            header::USER_AGENT,
        ];

        for expected_header in expected_headers {
            assert!(FORWARD_HEADERS.contains(&expected_header));
        }
    }

    #[test]
    fn test_no_cache_exts_contains_expected_extensions() {
        // 测试 NO_CACHE_EXTS 包含预期的扩展名
        assert!(NO_CACHE_EXTS.contains(&"html"));
        assert!(NO_CACHE_EXTS.contains(&"htm"));
        assert!(!NO_CACHE_EXTS.contains(&"css"));
        assert!(!NO_CACHE_EXTS.contains(&"js"));
    }

    #[test]
    fn test_cache_control_value_format() {
        // 测试缓存控制值的格式
        assert_eq!(CACHE_CONTROL_VALUE, "public, max-age=2592000");
        assert!(CACHE_CONTROL_VALUE.contains("public"));
        assert!(CACHE_CONTROL_VALUE.contains("max-age=2592000"));
    }

    #[tokio::test]
    async fn test_handle_files_empty_path_redirects() {
        // 测试空路径重定向
        let _req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        // 由于重定向逻辑，这里需要更复杂的测试设置
        // 实际测试中应该使用 mock 或测试专用的 S3 客户端
    }

    #[test]
    fn test_header_value_constants() {
        // 测试常量头部值可以正确创建
        let cache_control_value = HeaderValue::from_static(CACHE_CONTROL_VALUE);
        assert!(cache_control_value.as_bytes().is_ascii());
        assert_eq!(cache_control_value.to_str().unwrap(), CACHE_CONTROL_VALUE);
    }

    #[test]
    fn test_path_trimming_logic() {
        // 测试路径修剪逻辑
        let test_cases = vec![
            ("/", ""),
            ("/index.html", "index.html"),
            ("/subdir/", "subdir"),
            ("/subdir/index.html", "subdir/index.html"),
            ("", ""),
        ];

        for (input, expected) in test_cases {
            let trimmed = input.trim_start_matches('/').trim_end_matches('/');
            assert_eq!(trimmed, expected);
        }
    }
}
