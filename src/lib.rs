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
use axum::routing::get;
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
}

/// 创建并配置Axum应用程序
///
/// 此函数设置了一个完整的HTTP服务器，包括：
/// - CORS配置，允许GET、HEAD和OPTIONS请求
/// - S3客户端初始化和集成
/// - 请求追踪中间件
/// - 文件处理路由配置
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

    // 创建应用状态
    let state = AppState {
        s3_client,
        http_client,
    };

    axum::Router::new()
        .fallback(get(handlers::files::handle_files))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
