use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use reqwest::Method;

use crate::AppState;
use crate::utils::proxy::proxy_request;

/// 处理用户余额查询请求
///
/// 将客户端的 GET /free-model/user/balance 请求代理转发到 DeepSeek API 的 /user/balance 接口。
/// 该接口用于查询当前 API 密钥的账户余额信息。
///
/// # 请求方法
///
/// GET /free-model/user/balance
///
/// # 参数
///
/// * `state` - 应用状态，包含 HTTP 客户端和 API 密钥
/// * `headers` - 客户端传入的请求头，会被过滤后转发
///
/// # 返回值
///
/// * `Ok(Response)` - DeepSeek API 返回的余额信息（JSON 格式）
/// * `Err((StatusCode, String))` - 代理失败时的错误信息
///
/// # 响应示例
///
/// 成功时返回类似：
/// ```json
/// {
///   "available_balance": "100.00",
///   "currency": "USD",
///   "total_balance": "100.00"
/// }
/// ```
pub async fn handle_balance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    // 应用层处理认证：如果客户端未提供 AUTHORIZATION，则添加服务器的 Bearer token
    let mut request_headers = headers;
    if !request_headers.contains_key(header::AUTHORIZATION) {
        let auth_value = axum::http::HeaderValue::from_str(&format!("Bearer {}", state.deepseek_api_key))
            .map_err(|e: axum::http::header::InvalidHeaderValue| {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;
        request_headers.insert(header::AUTHORIZATION, auth_value);
    }

    proxy_request(
        &state.http_client,
        "https://api.deepseek.com/user/balance",
        Method::GET,
        request_headers,
        None,
        None,
    )
    .await
}
