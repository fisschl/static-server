use static_server::app;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app = app().await;

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    tracing::info!("Server running on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service(),
    )
    .await?;

    Ok(())
}
