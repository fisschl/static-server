use actix_cors::Cors;
use actix_web::{App, HttpServer};

// 导入我们的模块
mod config;
mod handlers;
mod s3;

use handlers::files;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    let addr = "0.0.0.0:3000";

    println!("服务器运行在 http://{}", addr);

    HttpServer::new(move || {
        let cors = Cors::permissive()
            .allowed_methods(vec!["GET", "HEAD", "OPTIONS"])
            .allowed_headers(vec!["*"]);

        App::new()
            .wrap(cors)
            .default_service(actix_web::web::get().to(files))
    })
    .bind(addr)?
    .run()
    .await
}
