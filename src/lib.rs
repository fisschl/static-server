pub mod handlers;
pub mod s3;

use axum::routing::get;
use http::Method;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, CorsLayer};
use tower_http::trace::TraceLayer;

pub async fn app() -> axum::Router {
    // 配置 CORS
    let cors = CorsLayer::permissive()
        .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS])
        .allow_headers(AllowHeaders::any());

    // 初始化 S3 客户端
    // AWS_ACCESS_KEY_ID=your-access-key-id
    // AWS_SECRET_ACCESS_KEY=your-access-key-secret
    // AWS_REGION=cn-hangzhou
    // AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
    let s3_config = aws_config::load_from_env().await;
    let s3_client = Arc::new(aws_sdk_s3::Client::new(&s3_config));

    axum::Router::new()
        .fallback(get(handlers::handle_files))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::extract::Extension(s3_client))
}
