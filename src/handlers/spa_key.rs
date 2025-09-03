use moka::future::Cache;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

/// 创建一个静态缓存实例，用于缓存find_exists_key的结果
/// 缓存配置：
/// - 最大容量：32768个条目（32 * 1024）
/// - 默认过期时间：60秒
static PATH_EXISTS_CACHE: Lazy<Arc<Cache<String, Option<String>>>> = Lazy::new(|| {
    Arc::new(
        Cache::builder()
            .max_capacity(32 * 1024)
            .time_to_live(Duration::from_secs(60))
            .build(),
    )
});

/// 带缓存的查找请求文件的 S3 键。
///
/// 此函数是find_exists_key的缓存版本，使用moka缓存来避免重复的 S3 请求。
///
/// # 参数
///
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
pub async fn find_exists_key_with_cache(pathname: &str) -> Option<String> {
    // 转换为 String 以便在缓存中使用
    let path_str = pathname.to_string();

    // 首先检查缓存
    if let Some(result) = PATH_EXISTS_CACHE.get(&path_str).await {
        return result.clone();
    }

    // 计算结果
    let result = find_exists_key(pathname).await;

    // 将结果存入缓存并返回
    PATH_EXISTS_CACHE.insert(path_str, result.clone()).await;
    result
}

use crate::s3::config::{get_bucket_name, get_s3_client};

/// 检查 S3 存储桶中是否存在指定键。
///
/// # 参数
///
/// * `key` - 要检查的 S3 键。
///
/// # 返回值
///
/// 如果键存在则返回 `true`，否则返回 `false`。
pub async fn check_key_exists(key: &str) -> bool {
    // 执行实际的 S3 检查
    let s3_client = get_s3_client().await;
    let bucket_name = get_bucket_name();

    let result = s3_client
        .head_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;

    result.is_ok()
}

/// 默认的索引文件名
const INDEX_FILE: &str = "index.html";

/// 查找请求文件的 S3 键。
///
/// 此函数实现了 SPA 支持的回退逻辑：
/// - 检查第一级目录中的 index.html。
///
/// # 参数
///
/// * `pathname` - 请求的文件路径。
///
/// # 返回值
///
/// 要提供的文件的 S3 键，如果未找到文件则返回 `None`。
pub async fn find_exists_key(pathname: &str) -> Option<String> {
    // 1. 检查第一级目录中的 index.html
    // 获取第一级目录（只处理正斜杠，因为 URL 总是使用正斜杠）
    let first_level_dir = pathname.split('/').next().unwrap_or("");
    if !first_level_dir.is_empty() {
        let first_level_index = format!("{}/{}", first_level_dir, INDEX_FILE);
        if check_key_exists(&first_level_index).await {
            return Some(first_level_index);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use uuid::Uuid;

    async fn create_test_directory() -> Result<String> {
        let s3_client = get_s3_client().await;
        let bucket_name = get_bucket_name();

        // 使用uuidv7生成唯一的目录名
        let dir_name = Uuid::now_v7().to_string();

        // 创建index.html文件
        let index_content = "<html><body><h1>Test Index</h1></body></html>";
        let index_key = format!("{}/index.html", dir_name);

        // 创建另一个测试文件
        let other_content = "console.log('test file');";
        let other_key = format!("{}/test.js", dir_name);

        // 上传文件到S3
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
            .key(&other_key)
            .body(other_content.as_bytes().to_owned().into())
            .send()
            .await?;

        println!("创建测试目录: {}，包含 index.html 和 test.js", dir_name);

        Ok(dir_name)
    }

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

        println!("清理测试目录: {}", dir_name);

        Ok(())
    }

    /// 测试目录守卫，使用Drop trait自动清理测试资源
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

    impl std::ops::Deref for TestDirectoryGuard {
        type Target = String;
        
        fn deref(&self) -> &Self::Target {
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
                        eprintln!("清理测试目录 {} 时出错: {}", dir_name, e);
                    }
                });
            });
        }
    }

    #[tokio::test]
    /// 测试SPA回退逻辑
    ///
    /// 验证单页面应用的回退机制，
    /// 当访问不存在的路径时能正确回退到index.html
    async fn test_spa_fallback_logic_with_real_files() {
        // 使用测试目录守卫自动清理
        let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
        let test_dir = test_dir_guard.dir_name();

        // 测试SPA回退逻辑
        let dir_index_path = format!("{}/index.html", test_dir);

        // 测试访问不存在的路径时是否回退到目录的index.html
        if check_key_exists(&dir_index_path).await {
            // 测试访问该目录下的不存在的页面路径
            let non_existent_page = format!("{}/about", test_dir);
            let result = find_exists_key(&non_existent_page).await;
            // 应该回退到目录的index.html
            assert!(
                result.is_some(),
                "访问不存在的路径时应该回退到目录的index.html"
            );
            assert_eq!(result.unwrap(), dir_index_path);

            // 测试访问不存在的嵌套路径
            let non_existent_nested = format!("{}/user/profile", test_dir);
            let result2 = find_exists_key(&non_existent_nested).await;
            assert!(
                result2.is_some(),
                "访问不存在的嵌套路径时应该回退到目录的index.html"
            );
            assert_eq!(result2.unwrap(), dir_index_path);
        }
    }

    #[tokio::test]
    /// 测试一级目录index.html回退逻辑
    ///
    /// 验证当访问不存在的路径但其一级子目录下存在index.html时，
    /// 系统能正确回退到该目录的index.html文件
    async fn test_first_level_directory_index_fallback() {
        // 使用测试目录守卫自动清理
        let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
        let test_dir = test_dir_guard.dir_name();

        let dir_index_path = format!("{}/index.html", test_dir);

        // 测试用例1: 访问该目录下的不存在的具体页面
        let non_existent_page = format!("{}/nonexistent-page", test_dir);
        let result = find_exists_key(&non_existent_page).await;

        // 应该回退到该目录的index.html
        assert!(
            result.is_some(),
            "访问不存在的页面 {} 时应该回退到 {}/index.html",
            non_existent_page,
            test_dir
        );
        assert_eq!(
            result.unwrap(),
            dir_index_path,
            "应该返回正确的目录index.html路径"
        );

        println!(
            "成功测试: 访问 {} 回退到 {}",
            non_existent_page, dir_index_path
        );
    }

    #[tokio::test]
    /// 测试多种路径模式的一级目录回退
    ///
    /// 验证测试目录下各种不存在的路径模式
    /// 都能正确回退到目录的index.html文件
    async fn test_first_level_directory_various_paths() {
        // 使用测试目录守卫自动清理
        let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
        let test_dir = test_dir_guard.dir_name();

        let dir_index_path = format!("{}/index.html", test_dir);

        // 测试多种不存在的路径模式
        let test_paths = [
            format!("{}/about", test_dir),
            format!("{}/user/profile", test_dir),
            format!("{}/products/123", test_dir),
            format!("{}/blog/post-title", test_dir),
        ];

        for path in test_paths.iter() {
            let result = find_exists_key(path).await;

            // 所有不存在的路径都应该回退到目录的index.html
            // 注意: 如果路径对应的文件实际存在，则不会回退
            if check_key_exists(path).await {
                // 如果文件存在，应该返回文件本身而不是回退路径
                assert!(result.is_some(), "文件 {} 存在，应该返回文件本身", path);
            } else {
                // 如果文件不存在，应该回退到目录的index.html
                assert!(
                    result.is_some(),
                    "访问不存在的路径 {} 时应该回退到 {}/index.html",
                    path,
                    test_dir
                );
                assert_eq!(
                    result.unwrap(),
                    dir_index_path,
                    "应该返回正确的目录index.html路径"
                );
            }

            println!("成功测试: 访问 {} 回退到 {}", path, dir_index_path);
        }
    }

    #[tokio::test]
    /// 测试直接目录访问的回退逻辑
    ///
    /// 验证直接访问测试目录和测试目录/路径时
    /// 能正确返回对应目录的index.html文件
    async fn test_direct_directory_access() {
        // 使用测试目录守卫自动清理
        let test_dir_guard = TestDirectoryGuard::new().await.unwrap();
        let test_dir = test_dir_guard.dir_name();

        let dir_index_path = format!("{}/index.html", test_dir);

        // 测试直接访问目录（无斜杠）
        let result1 = find_exists_key(&test_dir).await;
        assert!(
            result1.is_some(),
            "直接访问 {} 目录应该返回index.html",
            test_dir
        );
        assert_eq!(
            result1.unwrap(),
            dir_index_path,
            "应该返回正确的目录index.html路径"
        );

        // 测试直接访问目录（带斜杠）
        let dir_with_slash = format!("{}/", test_dir);
        let result2 = find_exists_key(&dir_with_slash).await;
        assert!(
            result2.is_some(),
            "直接访问 {}/ 目录应该返回index.html",
            test_dir
        );
        assert_eq!(
            result2.unwrap(),
            dir_index_path,
            "应该返回正确的目录index.html路径"
        );

        println!(
            "成功测试: 直接访问 {} 和 {}/ 都能正确返回index.html",
            test_dir, test_dir
        );
    }
}
