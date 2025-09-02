use axum::{routing::get, Router};
use tower_http::cors::{CorsLayer, AllowHeaders};
use tower_http::trace::TraceLayer;
use std::net::SocketAddr;

// 导入我们的模块
mod s3;

use s3::serve_files;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    // 初始化 tracing
    tracing_subscriber::fmt::init();

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;

    // 配置 CORS
    let cors = CorsLayer::permissive()
        .allow_methods([
            http::Method::GET,
            http::Method::HEAD,
            http::Method::OPTIONS,
        ])
        .allow_headers(AllowHeaders::any());

    let app = Router::new()
        .fallback(get(serve_files))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    tracing::info!("Server running on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service()
    ).await?;

    Ok(())
}
