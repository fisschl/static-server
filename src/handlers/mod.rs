//! 静态文件服务器的 HTTP 处理模块。
//!
//! 该模块包含用于从 S3 提供文件服务的主要请求处理函数。

use crate::config::generate_presigned_url;
use crate::s3::{find_exists_key, should_cache};
use actix_web::{HttpRequest, HttpResponse, Result};
use awc::Client;

/// 处理文件请求。
///
/// 此函数尝试在 S3 存储桶中查找请求的文件。如果未找到文件，
/// 它会实现回退机制来为 SPA 支持提供 `index.html`。
///
/// # 参数
///
/// * `req` - HTTP 请求。
///
/// # 返回值
///
/// 包含文件内容或错误状态的 HTTP 响应。
pub async fn files(req: HttpRequest) -> Result<HttpResponse> {
    let path = req.path().trim_start_matches('/');
    let pathname = if path.is_empty() { "" } else { path };

    let file_key = match find_exists_key(pathname).await {
        Some(key) => key,
        None => {
            return Ok(HttpResponse::NotFound().body("Not Found"));
        }
    };

    let file_ext = match file_key.rfind('.') {
        Some(idx) => &file_key[idx..],
        None => "",
    };

    // 生成预签名 URL
    match generate_presigned_url(&file_key, 3600).await { // 1小时过期
        Ok(presigned_url) => {
            // 使用 awc 客户端转发请求
            let client = Client::default();
            
            // 构建转发请求
            let mut forwarded_req = client.get(&presigned_url);
            
            // 只复制与代理相关的请求头部
            let forward_headers = ["accept", "accept-encoding", "range", "if-match", "if-none-match", 
                                  "if-modified-since", "if-unmodified-since", "user-agent"];
            
            // 复制原始请求的必要头部
            for (name, value) in req.headers() {
                let name_str = name.as_str().to_lowercase();
                if forward_headers.contains(&name_str.as_str()) {
                    forwarded_req = forwarded_req.insert_header((name.as_str(), value.as_bytes()));
                }
            }
            
            // 发送请求并获取响应
            match forwarded_req.send().await {
                Ok(response) => {
                    // 构建返回的响应
                    let mut resp = HttpResponse::build(response.status());
                    
                    // 复制必要的响应头部
                    for (name, value) in response.headers() {
                        let name_str = name.as_str().to_lowercase();
                        // 跳过可能引起问题的头部
                        if !["connection", "transfer-encoding", "server", "date"].contains(&name_str.as_str()) {
                            resp.insert_header((name.as_str(), value.as_bytes()));
                        }
                    }
                    
                    // 添加缓存控制头部
                    if should_cache(file_ext) && !response.headers().contains_key("cache-control") {
                        resp.insert_header(("cache-control", "public, max-age=2592000"));
                    }
                    
                    // 流式传输响应体
                    Ok(resp.streaming(response))
                }
                Err(err) => {
                    eprintln!("转发请求时出错: {:?}", err);
                    Ok(HttpResponse::InternalServerError().body("Internal Server Error"))
                }
            }
        }
        Err(err) => {
            eprintln!("生成预签名 URL 时出错: {:?}", err);
            Ok(HttpResponse::InternalServerError().body("Internal Server Error"))
        }
    }
}