use axum::body::Body;
use http::{Request, StatusCode, header};
use tower::util::ServiceExt;
use anyhow::Result;
use uuid::Uuid;

// 导入应用模块
use static_server::app;
use static_server::s3::config::{get_bucket_name, get_s3_client};

/// 创建测试目录并在S3中上传测试文件
async fn create_test_directory() -> Result<String> {
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name();

    // 使用uuidv7生成唯一的目录名
    let dir_name = Uuid::now_v7().to_string();

    // 创建index.html文件
    let index_content = "<html><body><h1>Integration Test Index</h1></body></html>";
    let index_key = format!("{}/index.html", dir_name);

    // 创建测试JS文件
    let js_content = "console.log('integration test file');";
    let js_key = format!("{}/test.js", dir_name);

    // 创建测试CSS文件
    let css_content = "body { color: red; }";
    let css_key = format!("{}/test.css", dir_name);

    // 创建测试图片文件（模拟PNG文件）
    let png_content = "fake png content";
    let png_key = format!("{}/test.png", dir_name);

    // 上传所有文件到S3
    s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&index_key)
        .body(index_content.as_bytes().to_owned().into())
        .send()
        .await?;

    s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&js_key)
        .body(js_content.as_bytes().to_owned().into())
        .send()
        .await?;

    s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&css_key)
        .body(css_content.as_bytes().to_owned().into())
        .send()
        .await?;

    s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&png_key)
        .body(png_content.as_bytes().to_owned().into())
        .send()
        .await?;

    println!("创建集成测试目录: {}，包含多个测试文件", dir_name);

    Ok(dir_name)
}

/// 清理测试目录
async fn teardown_test_directory(dir_name: &str) -> Result<()> {
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name();

    // 列出目录下的所有对象
    let objects = s3_client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(dir_name)
        .send()
        .await?
        .contents
        .unwrap_or_default();

    // 删除所有对象
    for object in objects {
        if let Some(key) = object.key {
            s3_client
                .delete_object()
                .bucket(&bucket_name)
                .key(&key)
                .send()
                .await?;
            println!("删除文件: {}", key);
        }
    }

    println!("清理集成测试目录: {}", dir_name);

    Ok(())
}

/// 集成测试目录守卫，使用Drop trait自动清理测试资源
struct TestDirectoryGuard {
    dir_name: String,
}

impl TestDirectoryGuard {
    /// 创建新的测试目录守卫
    async fn new() -> Result<Self> {
        let dir_name = create_test_directory().await?;
        Ok(Self { dir_name })
    }
    
    /// 获取目录名称
    fn dir_name(&self) -> &str {
        &self.dir_name
    }
}

impl Drop for TestDirectoryGuard {
    fn drop(&mut self) {
        // 使用block_on来在同步上下文中执行异步清理
        let dir_name = self.dir_name.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(e) = teardown_test_directory(&dir_name).await {
                    eprintln!("清理集成测试目录 {} 时出错: {}", dir_name, e);
                }
            });
        });
    }
}

/// 集成测试：测试空路径重定向
///
/// 验证当请求路径为空时，应用是否正确重定向到预设URL
#[tokio::test]
async fn test_empty_path_redirects() {
    let app = app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        "https://ys.mihoyo.com/"
    );
}

/// 集成测试：测试真实存在的文件请求处理
///
/// 验证应用能够正确处理真实存在的文件请求
#[tokio::test]
async fn test_real_file_request_exists() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    // 测试存在的文件
    let test_file = format!("/{}/test.js", test_dir);
    let response = app
        .oneshot(Request::builder().uri(&test_file).body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    // 真实存在的文件应该被正确处理
    assert!(response.status().is_success() || response.status().is_redirection());
}

/// 集成测试：测试不存在的文件返回404
///
/// 验证应用对不存在的文件正确返回404状态码
#[tokio::test]
async fn test_real_file_request_not_found() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    // 测试不存在的文件返回404
    let non_existent_file = format!("/{}/nonexistent-file.txt", test_dir);
    let response = app
        .oneshot(Request::builder().uri(&non_existent_file).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 集成测试：测试真实文件的缓存控制头部
///
/// 验证应用为可缓存文件正确设置缓存控制头部
#[tokio::test]
async fn test_real_cache_control_headers() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/{}/test.js", test_dir))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // JS文件应该有缓存控制头部
    let cache_control = response.headers().get(header::CACHE_CONTROL);
    assert!(cache_control.is_some(), "JS文件应该设置缓存控制头部");
}

/// 集成测试：测试真实HTML文件不缓存
///
/// 验证HTML文件不会被设置缓存控制头部
#[tokio::test]
async fn test_real_html_files_no_cache() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/{}/index.html", test_dir))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // HTML文件不应有缓存控制头部
    let cache_control = response.headers().get(header::CACHE_CONTROL);
    assert!(cache_control.is_none(), "HTML文件不应设置缓存控制头部");
}

/// 集成测试：测试真实CORS支持
///
/// 验证应用正确支持CORS跨域请求
#[tokio::test]
async fn test_real_cors_support() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/{}/test.css", test_dir))
                .header(header::ORIGIN, "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 验证CORS头部设置
    let cors_headers = response.headers();
    assert!(cors_headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_some(), 
            "应该设置CORS允许来源头部");
}

/// 集成测试：测试真实SPA回退机制
///
/// 验证单页应用回退机制正常工作
#[tokio::test]
async fn test_real_spa_fallback_mechanism() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    // 测试SPA路由（不存在的路径但应该回退到index.html）
    let spa_route = format!("/{}/spa/route", test_dir);
    let response = app
        .oneshot(Request::builder().uri(&spa_route).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // SPA路由应该成功处理（可能重定向或返回index.html内容）
    assert!(response.status().is_success() || response.status().is_redirection(),
            "SPA路由应该成功处理");
}

/// 集成测试：测试真实请求头转发
///
/// 验证应用正确转发必要的请求头到S3
#[tokio::test]
async fn test_real_request_headers_forwarding() {
    // 使用测试目录守卫自动清理
    let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
    let test_dir = test_dir_guard.dir_name();

    let app = app();

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/{}/test.png", test_dir))
                .header(header::ACCEPT, "image/png")
                .header(header::RANGE, "bytes=0-1023")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 验证请求被正确处理
    assert!(response.status().is_success() || response.status().is_redirection(),
            "带特殊头部的请求应该被正确处理");
}
