//! 静态文件服务器库
//!
//! 这是一个基于Axum的静态文件服务器，主要功能包括：
//! - 从S3存储桶服务静态文件
//! - 支持SPA(Single Page Application)路由
//! - 提供文件缓存和代理功能
//! - 支持CORS跨域请求

pub mod handlers;
pub mod utils;

use aws_sdk_s3::Client as S3Client;
use axum::routing::{get, post};
use reqwest::Client;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// 应用状态，包含所有共享资源
#[derive(Clone)]
pub struct AppState {
    /// S3 客户端实例
    pub s3_client: Arc<S3Client>,
    /// HTTP 客户端实例（用于代理请求）
    pub http_client: Arc<Client>,
    /// S3 存储桶名称
    pub bucket_name: String,
    /// DeepSeek API 密钥
    pub deepseek_api_key: String,
}

/// 创建并配置Axum应用程序
///
/// # Returns
///
/// 返回配置好的Axum Router实例
pub async fn app() -> axum::Router {
    // 初始化 S3 客户端
    let s3_config = aws_config::load_from_env().await;
    let s3_client = Arc::new(aws_sdk_s3::Client::new(&s3_config));

    // 初始化 HTTP 客户端用于代理
    let http_client = Arc::new(Client::new());

    // 从环境变量读取 S3 存储桶名称
    let bucket_name = std::env::var("AWS_BUCKET").expect(
        "AWS_BUCKET environment variable must be set. Please set AWS_BUCKET=your-bucket-name",
    );

    // 从环境变量读取 DeepSeek API 密钥
    let deepseek_api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY environment variable must be set");

    // 创建应用状态
    let state = AppState {
        s3_client,
        http_client,
        bucket_name,
        deepseek_api_key,
    };

    let free_model_api_routes = axum::Router::new()
        .route(
            "/chat/completions",
            post(handlers::chat_completions::handle_chat_completions),
        )
        .route("/models", get(handlers::models::handle_models))
        .route("/user/balance", get(handlers::balance::handle_balance));

    axum::Router::new()
        .nest("/free-model", free_model_api_routes)
        .fallback(get(handlers::files::handle_files))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
