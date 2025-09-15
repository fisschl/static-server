use dotenv::dotenv;
use static_server::app;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(Level::DEBUG)
        .init();

    let app = app().await;

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    tracing::info!("Server running on {}", addr);

    axum::serve(TcpListener::bind(addr).await?, app.into_make_service()).await?;

    Ok(())
}
