pub mod handlers;
pub mod s3;

// 创建应用Router
use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;

pub async fn app() -> axum::Router {
    dotenv::dotenv().ok();

    use axum::routing::get;
    use tower_http::cors::{AllowHeaders, CorsLayer};
    use tower_http::trace::TraceLayer;

    // 配置 CORS
    let cors = CorsLayer::permissive()
        .allow_methods([http::Method::GET, http::Method::HEAD, http::Method::OPTIONS])
        .allow_headers(AllowHeaders::any());

    // 初始化 S3 客户端
    // AWS_ACCESS_KEY_ID=your-access-key-id
    // AWS_SECRET_ACCESS_KEY=your-access-key-secret
    // AWS_REGION=cn-hangzhou
    // AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
    let s3_config = aws_config::load_from_env().await;
    let s3_client: Arc<S3Client> = Arc::new(S3Client::new(&s3_config));

    axum::Router::new()
        .fallback(get(handlers::handle_files))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::extract::Extension(s3_client))
}
