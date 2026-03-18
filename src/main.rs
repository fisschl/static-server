use static_server::app;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::Level;
use tracing_subscriber::fmt::time::LocalTime;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .pretty()
        .with_timer(LocalTime::rfc_3339())
        .with_max_level(Level::DEBUG)
        .init();

    let app = app().await;

    let addr: SocketAddr = "0.0.0.0:3000"
        .parse()
        .expect("Failed to parse socket address: invalid address format");
    tracing::info!("Server running on {}", addr);

    axum::serve(TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
