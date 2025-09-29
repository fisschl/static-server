use super::proxy::fetch_and_proxy_file;
use super::spa_key;
use super::constants::WWW_PREFIX;
use crate::utils::s3::get_bucket_name;
use aws_sdk_s3::Client as S3Client;
use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use std::sync::Arc;

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

    // 在 /www 前缀下查找文件
    let s3_path = format!("{}/{}", WWW_PREFIX, path);
    
    // 尝试直接获取请求的文件
    match fetch_and_proxy_file(s3_client.clone(), req.headers(), &s3_path).await {
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
    let file_key = match spa_key::find_exists_key(s3_client.clone(), &bucket_name, path).await {
        Some(key) => key,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // 使用 fetch_and_proxy_file 获取回退文件
    match fetch_and_proxy_file(s3_client, req.headers(), &file_key).await {
        Ok(response) => response.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}
