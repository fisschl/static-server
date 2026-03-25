use axum_test::TestServer;
use static_server::{app_with_deps, storage::{MockStorage, KeyStatus}};
use mockall::predicate::*;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_file_success() {
    let mock_server = MockServer::start().await;
    let mut mock_storage = MockStorage::new();

    // Mock 存储返回指向 Wiremock 的预签名 URL
    let mock_uri = mock_server.uri();
    mock_storage
        .expect_get_presigned_url()
        .with(eq("test-bucket"), eq("www/index.html"))
        .returning(move |_, _| Ok(format!("{}/index.html", mock_uri)));

    // Wiremock 模拟 S3 返回文件内容
    Mock::given(method("GET"))
        .and(path("/index.html"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("content-type", "text/html")
            .set_body_string("<html>Hello</html>"))
        .mount(&mock_server)
        .await;

    let http_client = reqwest::Client::new();
    let app = app_with_deps(mock_storage, http_client, "test-bucket".into());
    let server = TestServer::new(app).unwrap();

    let response = server.get("/index.html").await;
    response.assert_status_ok();
    response.assert_text("<html>Hello</html>");
}

#[tokio::test]
async fn test_get_file_not_found() {
    let mut mock_storage = MockStorage::new();

    // 文件不存在
    mock_storage
        .expect_get_presigned_url()
        .returning(|_, _| Err(static_server::error::AppError::NotFound));

    // SPA fallback 也找不到
    mock_storage
        .expect_check_key_exists()
        .returning(|_, _| KeyStatus::NotFound);

    let http_client = reqwest::Client::new();
    let app = app_with_deps(mock_storage, http_client, "test-bucket".into());
    let server = TestServer::new(app).unwrap();

    let response = server.get("/nonexistent.txt").await;
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_spa_fallback() {
    let mock_server = MockServer::start().await;
    let mut mock_storage = MockStorage::new();
    let mock_uri = mock_server.uri();

    // 首先尝试直接获取 /app/page，HTTP 返回 404 触发 fallback
    mock_storage
        .expect_get_presigned_url()
        .with(eq("test-bucket"), eq("www/app/page"))
        .returning({
            let uri = mock_uri.clone();
            move |_, _| Ok(format!("{}/app/page", uri))
        });

    // 第一次 HTTP 请求返回 404
    Mock::given(method("GET"))
        .and(path("/app/page"))
        .respond_with(ResponseTemplate::new(404)
            .set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    // 模拟 /app/page 不存在，但 /app/index.html 存在
    mock_storage
        .expect_check_key_exists()
        .with(eq("test-bucket"), eq("www/app/page/index.html"))
        .returning(|_, _| KeyStatus::NotFound);

    mock_storage
        .expect_check_key_exists()
        .with(eq("test-bucket"), eq("www/app/index.html"))
        .returning(|_, _| KeyStatus::Exists);

    mock_storage
        .expect_get_presigned_url()
        .with(eq("test-bucket"), eq("www/app/index.html"))
        .returning(move |_, _| Ok(format!("{}/app/index.html", mock_uri)));

    Mock::given(method("GET"))
        .and(path("/app/index.html"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("SPA App"))
        .mount(&mock_server)
        .await;

    let http_client = reqwest::Client::new();
    let app = app_with_deps(mock_storage, http_client, "test-bucket".into());
    let server = TestServer::new(app).unwrap();

    // 访问 /app/page 应该 fallback 到 /app/index.html
    let response = server.get("/app/page").await;
    response.assert_status_ok();
    response.assert_text("SPA App");
}
