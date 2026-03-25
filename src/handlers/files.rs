use crate::error::AppError;
use crate::storage::{KeyStatus, Storage};
use crate::utils::headers::filter_headers_blacklist;
use crate::utils::headers::guess_mime_type;
use crate::utils::path::get_extension_lowercase;
use crate::utils::proxy::{REQUEST_HEADERS_BLOCKLIST, RESPONSE_HEADERS_BLOCKLIST};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};

/// S3 存储桶中的 www 前缀
pub const WWW_PREFIX: &str = "www";

/// 默认的索引文件名
pub const INDEX_FILE: &str = "index.html";

/// 不应缓存的文件扩展名。
pub const NO_CACHE_EXTS: &[&str] = &["html", "htm"];

/// 缓存控制头部值（30 天缓存，适用于 CSS、JS、图片等静态资源）
/// max-age=2592000 表示 2592000 秒 = 30 天
pub const CACHE_CONTROL_VALUE: &str = "public, max-age=2592000";

/// 确定文件键是否应该被缓存。
///
/// # 参数
///
/// * `key` - 要检查的文件键。
///
/// # 返回值
///
/// 如果文件应该被缓存则返回 `true`，否则返回 `false`。
pub fn should_cache(key: &str) -> bool {
    let ext = get_extension_lowercase(key);
    !NO_CACHE_EXTS.contains(&ext.as_str())
}

/// 从存储获取文件内容并返回响应
pub async fn fetch_and_proxy_file(
    storage: &dyn Storage,
    http_client: &reqwest::Client,
    bucket_name: &str,
    headers: &http::HeaderMap,
    key: &str,
) -> Result<Response<Body>, AppError> {
    let presigned_url = storage.get_presigned_url(bucket_name, key).await?;

    let forwarded_headers = filter_headers_blacklist(headers, REQUEST_HEADERS_BLOCKLIST);

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

    let filtered_headers = filter_headers_blacklist(&response_headers, &RESPONSE_HEADERS_BLOCKLIST);
    for (name, value) in filtered_headers.iter() {
        resp_builder = resp_builder.header(name, value);
    }

    if !response_headers.contains_key(header::CONTENT_TYPE) {
        if let Some(guessed_content_type) = guess_mime_type(key) {
            resp_builder = resp_builder.header(header::CONTENT_TYPE, guessed_content_type);
        }
    }

    if status.is_success() && should_cache(key) {
        resp_builder = resp_builder.header(header::CACHE_CONTROL, CACHE_CONTROL_VALUE);
    }

    Ok(resp_builder.body(Body::from(body))?)
}

/// 检查存储中是否存在指定键
pub async fn check_key_exists(
    storage: &dyn Storage,
    bucket_name: &str,
    key: &str,
) -> KeyStatus {
    storage.check_key_exists(bucket_name, key).await
}

/// 查找请求文件的 S3 键
pub async fn find_exists_key(
    storage: &dyn Storage,
    bucket_name: &str,
    pathname: &str,
) -> Option<String> {
    let dir_index = format!("{WWW_PREFIX}/{}/{INDEX_FILE}", pathname);
    if storage.check_key_exists(bucket_name, &dir_index).await == KeyStatus::Exists {
        return Some(dir_index);
    }

    let parts: Vec<&str> = pathname.split('/').collect();
    for i in (1..parts.len()).rev() {
        let parent_path = parts[..i].join("/");
        let index_key = format!("{WWW_PREFIX}/{}/{INDEX_FILE}", parent_path);
        if check_key_exists(storage, bucket_name, &index_key).await == KeyStatus::Exists {
            return Some(index_key);
        }
    }

    let root_index = format!("{WWW_PREFIX}/{INDEX_FILE}");
    if check_key_exists(storage, bucket_name, &root_index).await == KeyStatus::Exists {
        return Some(root_index);
    }

    None
}

/// 处理文件请求
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
        &state.bucket_name,
        req.headers(),
        &s3_path,
    )
    .await?;

    if response.status() != StatusCode::NOT_FOUND {
        return Ok(response);
    }

    let file_key = find_exists_key(state.storage.as_ref(), &state.bucket_name, path)
        .await
        .ok_or(AppError::NotFound)?;

    fetch_and_proxy_file(
        state.storage.as_ref(),
        &state.http_client,
        &state.bucket_name,
        req.headers(),
        &file_key,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockStorage;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_fetch_and_proxy_file_success() {
        let mock_server = MockServer::start().await;
        let mut mock_storage = MockStorage::new();
        let mock_uri = mock_server.uri();

        Mock::given(method("GET"))
            .and(path("/test.txt"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("Hello World"))
            .mount(&mock_server)
            .await;

        mock_storage
            .expect_get_presigned_url()
            .returning(move |_, _| Ok(format!("{}/test.txt", mock_uri)));

        let http_client = reqwest::Client::new();

        let result = fetch_and_proxy_file(
            &mock_storage,
            &http_client,
            "test-bucket",
            &http::HeaderMap::new(),
            "www/test.txt",
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
    }

    #[test]
    fn test_should_cache() {
        assert!(should_cache("file.css"));
        assert!(should_cache("file.js"));
        assert!(should_cache("image.png"));
        assert!(!should_cache("page.html"));
        assert!(!should_cache("page.htm"));
    }
}
