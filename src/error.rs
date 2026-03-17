//! 应用错误类型定义
//!
//! 使用 thiserror 定义统一的错误类型，简化错误处理

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// 应用错误类型
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    /// S3 相关错误（包含操作和预签名）
    #[error("S3 error: {0}")]
    S3(String),

    /// HTTP 请求错误
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// 响应构建错误
    #[error("Failed to build response: {0}")]
    ResponseBuild(#[from] axum::http::Error),

    /// 文件未找到
    #[error("File not found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            // 502 Bad Gateway - 上游服务错误
            AppError::S3(_) | AppError::Http(_) => {
                (StatusCode::BAD_GATEWAY, self.to_string()).into_response()
            }
            // 404 Not Found
            AppError::NotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string()).into_response()
            }
            // 500 Internal Server Error
            AppError::ResponseBuild(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}
