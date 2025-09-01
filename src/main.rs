use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use env_logger::Env;

// 导入我们的模块
mod s3;

use s3::serve_files;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    let addr = "0.0.0.0:3000";

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(move || {
        let cors = Cors::permissive()
            .allowed_methods(vec!["GET", "HEAD", "OPTIONS"])
            .allowed_headers(vec!["*"]);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .default_service(actix_web::web::get().to(serve_files))
    })
    .bind(addr)?
    .run()
    .await
}
