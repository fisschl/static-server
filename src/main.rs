use axum::{Router, routing::get};
use std::net::SocketAddr;
use tower_http::cors::{AllowHeaders, CorsLayer};
use tower_http::trace::TraceLayer;

// 导入我们的模块
mod handlers;
mod s3;

use handlers::handle_files;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 tracing
    tracing_subscriber::fmt::init();

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;

    // 配置 CORS
    let cors = CorsLayer::permissive()
        .allow_methods([http::Method::GET, http::Method::HEAD, http::Method::OPTIONS])
        .allow_headers(AllowHeaders::any());

    let app = Router::new()
        .fallback(get(handle_files))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    tracing::info!("Server running on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service(),
    )
    .await?;

    Ok(())
}
