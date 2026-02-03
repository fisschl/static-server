use axum::{
    extract::{RawQuery, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::Response,
};

use crate::AppState;
use crate::utils::proxy::proxy_request;

/// 处理聊天补全请求
///
/// 将客户端的 POST /free-model/chat/completions 请求代理转发到 DeepSeek API 的 /chat/completions 接口。
/// 该接口用于生成对话补全，支持流式和非流式两种模式。
///
/// # 请求方法
///
/// POST /free-model/chat/completions[?query_string]
///
/// # 参数
///
/// * `state` - 应用状态，包含 HTTP 客户端和 API 密钥
/// * `query` - 可选的查询参数（如流式模式相关的参数）
/// * `method` - HTTP 请求方法（通常为 POST，但也支持其他方法）
/// * `headers` - 客户端传入的请求头，会被过滤后转发
/// * `body` - 请求体，包含对话消息、模型选择等参数（JSON 格式）
///
/// # 返回值
///
/// * `Ok(Response)` - DeepSeek API 返回的聊天补全结果（JSON 格式或流式文本）
/// * `Err((StatusCode, String))` - 代理失败时的错误信息
///
/// # 流式支持
///
/// 当请求中设置 `"stream": true` 时，API 会以 Server-Sent Events (SSE) 格式返回流式响应，
/// 本函数会以流式方式转发响应，保持连接直到所有数据传输完成。
pub async fn handle_chat_completions(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
    method: Method,
    headers: HeaderMap,
    body: Request,
) -> Result<Response, (StatusCode, String)> {
    // 应用层处理认证：如果客户端未提供 AUTHORIZATION，则添加服务器的 Bearer token
    let mut request_headers = headers;
    if !request_headers.contains_key(header::AUTHORIZATION) {
        let auth_str = format!("Bearer {}", state.deepseek_api_key);
        let auth_value = axum::http::HeaderValue::from_str(&auth_str)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        request_headers.insert(header::AUTHORIZATION, auth_value);
    }

    let body_stream = body.into_body().into_data_stream();

    proxy_request(
        &state.http_client,
        "https://api.deepseek.com/chat/completions",
        method,
        request_headers,
        query,
        Some(reqwest::Body::wrap_stream(body_stream)),
    )
    .await
}
