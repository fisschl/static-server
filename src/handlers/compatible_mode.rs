use crate::handlers::constants::{FORWARD_HEADERS, PRESERVE_HEADERS};
use crate::utils::headers::clone_headers;
use axum::{
    body::Body,
    extract::{Extension, Request},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use reqwest::Client;
use std::sync::Arc;

/// 处理compatible-mode请求的代理转发
///
/// 将以compatible-mode/v1开头的请求转发到https://dashscope.aliyuncs.com/compatible-mode/v1
/// 并流式返回响应
pub async fn handle_compatible_mode_proxy(
    Extension(client): Extension<Arc<Client>>,
    req: Request,
) -> impl IntoResponse {
    let path_query = match req.uri().path_and_query() {
        None => return (StatusCode::BAD_REQUEST, "Invalid request URI").into_response(),
        Some(path_query) => path_query.as_str(),
    };
    // 构建目标URL
    let target_url = format!("https://dashscope.aliyuncs.com{path_query}");

    // 获取请求方法、头部和请求体
    let method = req.method().clone();
    let headers = req.headers().clone();

    // 将 Axum Body 转换为 reqwest Body，保持流式特性
    // 即使是空流（如 GET 请求），也能正确处理
    let reqwest_body = reqwest::Body::wrap_stream(req.into_body().into_data_stream());

    // 构建转发请求，使用流式请求体
    let mut request_builder = client.request(method, &target_url).body(reqwest_body);

    // 复制请求头部（使用白名单方式，与 proxy.rs 保持一致）
    let forwarded_headers = clone_headers(&headers, FORWARD_HEADERS);
    request_builder = request_builder.headers(forwarded_headers);

    // 发送请求
    let response = match request_builder.send().await {
        Ok(response) => response,
        Err(e) => {
            let message = format!("Proxy request failed: {}", e);
            return (StatusCode::BAD_GATEWAY, message).into_response();
        }
    };

    let status = response.status();
    let mut response_builder = Response::builder().status(status);

    // 复制响应头部（使用与 proxy.rs 一致的 PRESERVE_HEADERS）
    for header_name in PRESERVE_HEADERS {
        if let Some(value) = response.headers().get(header_name) {
            response_builder = response_builder.header(header_name, value);
        }
    }

    // 流式传输响应体
    match response_builder.body(Body::from_stream(response.bytes_stream())) {
        Ok(resp) => resp.into_response(),
        Err(e) => {
            let message = format!("Failed to build response: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}
