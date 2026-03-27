//! 静态文件服务器库
//!
//! 这是一个基于Axum的静态文件服务器，主要功能包括：
//! - 从S3存储桶服务静态文件
//! - 支持SPA(Single Page Application)路由
//! - 提供文件缓存和代理功能
//! - 支持CORS跨域请求

pub mod error;
pub mod handlers;
pub mod storage;

use storage::{Storage, S3Storage};
use axum::routing::get;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// 应用状态 - 使用 Arc 包装 Storage trait 对象
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub http_client: reqwest::Client,
    pub bucket_name: String,
}

/// 创建应用（生产环境）
pub async fn app() -> axum::Router {
    // 初始化 S3 客户端
    let s3_config = aws_config::load_from_env().await;
    let s3_client = Arc::new(aws_sdk_s3::Client::new(&s3_config));
    let storage = S3Storage::new(s3_client);

    // 初始化 HTTP 客户端
    let http_client = reqwest::Client::new();

    // 从环境变量读取 S3 存储桶名称
    let bucket_name = std::env::var("AWS_BUCKET")
        .expect("AWS_BUCKET environment variable must be set");

    let state = AppState {
        storage: Arc::new(storage),
        http_client,
        bucket_name,
    };

    axum::Router::new()
        .fallback(get(handlers::files::handle_files))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

/// 创建应用（可注入存储和 HTTP 客户端，用于测试）
pub fn app_with_deps<S>(storage: S, http_client: reqwest::Client, bucket: String) -> axum::Router
where
    S: Storage + 'static,
{
    let state = AppState {
        storage: Arc::new(storage),
        http_client,
        bucket_name: bucket,
    };

    axum::Router::new()
        .fallback(get(handlers::files::handle_files))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
