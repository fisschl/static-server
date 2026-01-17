use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use reqwest::Method;

use crate::AppState;
use crate::utils::proxy::proxy_request;

/// 处理模型列表查询请求
///
/// 将客户端的 GET /free-model/models 请求代理转发到 DeepSeek API 的 /models 接口。
/// 该接口用于获取当前 API 密钥可访问的所有模型列表及其详细信息。
///
/// # 请求方法
///
/// GET /free-model/models
///
/// # 参数
///
/// * `state` - 应用状态，包含 HTTP 客户端和 API 密钥
/// * `headers` - 客户端传入的请求头，会被过滤后转发
///
/// # 返回值
///
/// * `Ok(Response)` - DeepSeek API 返回的模型列表（JSON 格式）
/// * `Err((StatusCode, String))` - 代理失败时的错误信息
///
/// # 响应示例
///
/// 成功时返回类似：
/// ```json
/// {
///   "object": "list",
///   "data": [
///     {
///       "id": "deepseek-chat",
///       "object": "model",
///       "owned_by": "deepseek",
///       "permission": []
///     }
///   ]
/// }
/// ```
pub async fn handle_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    // 应用层处理认证：如果客户端未提供 AUTHORIZATION，则添加服务器的 Bearer token
    let mut request_headers = headers;
    if !request_headers.contains_key(header::AUTHORIZATION) {
        let auth_value =
            axum::http::HeaderValue::from_str(&format!("Bearer {}", state.deepseek_api_key))
                .map_err(|e: axum::http::header::InvalidHeaderValue| {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                })?;
        request_headers.insert(header::AUTHORIZATION, auth_value);
    }

    proxy_request(
        &state.http_client,
        "https://api.deepseek.com/models",
        Method::GET,
        request_headers,
        None,
        None,
    )
    .await
}
