pub mod handlers;
pub mod s3;

// 创建应用Router
pub fn app() -> axum::Router {
    use axum::routing::get;
    use tower_http::cors::{AllowHeaders, CorsLayer};
    use tower_http::trace::TraceLayer;

    // 配置 CORS
    let cors = CorsLayer::permissive()
        .allow_methods([http::Method::GET, http::Method::HEAD, http::Method::OPTIONS])
        .allow_headers(AllowHeaders::any());

    axum::Router::new()
        .fallback(get(handlers::handle_files))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
