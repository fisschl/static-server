use axum::http::header;

/// S3 存储桶中的 www 前缀
pub const WWW_PREFIX: &str = "www";

/// 默认重定向 URL
pub const DEFAULT_REDIRECT_URL: &str = "https://ys.mihoyo.com/";

/// 默认的索引文件名
pub const INDEX_FILE: &str = "index.html";

/// 不应缓存的文件扩展名。
pub const NO_CACHE_EXTS: &[&str] = &["html", "htm"];

/// 需要保留的响应头部列表
pub const PRESERVE_HEADERS: &[header::HeaderName] = &[
    header::ACCEPT_RANGES,
    header::CACHE_CONTROL,
    header::CONTENT_DISPOSITION,
    header::CONTENT_ENCODING,
    header::CONTENT_LANGUAGE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
    header::CONTENT_TYPE,
    header::ETAG,
    header::EXPIRES,
    header::LAST_MODIFIED,
    header::VARY,
];

/// 缓存控制头部值
pub const CACHE_CONTROL_VALUE: &str = "public, max-age=2592000";

/// 用于代理的请求头部列表
///
/// 这些头部应该从客户端请求转发到目标服务器：
/// - 内容协商：ACCEPT_*
/// - 身份验证：AUTHORIZATION, PROXY_AUTHORIZATION
/// - 请求体信息：CONTENT_*
/// - 客户端信息：USER_AGENT, REFERER
/// - 条件请求：IF_*
/// - 范围请求：RANGE, IF_RANGE
/// - 其他：CACHE_CONTROL, PRAGMA, COOKIE
pub const FORWARD_HEADERS: &[header::HeaderName] = &[
    // 内容协商
    header::ACCEPT,
    header::ACCEPT_CHARSET,
    header::ACCEPT_ENCODING,
    header::ACCEPT_LANGUAGE,
    // 身份验证
    header::AUTHORIZATION,
    header::PROXY_AUTHORIZATION,
    // 请求体相关
    header::CONTENT_TYPE,
    header::CONTENT_LENGTH,
    header::CONTENT_ENCODING,
    header::CONTENT_LANGUAGE,
    // 客户端信息
    header::USER_AGENT,
    header::REFERER,
    // 条件请求
    header::IF_MATCH,
    header::IF_NONE_MATCH,
    header::IF_MODIFIED_SINCE,
    header::IF_UNMODIFIED_SINCE,
    // 范围请求
    header::RANGE,
    header::IF_RANGE,
    // 其他重要头部
    header::CACHE_CONTROL,
    header::PRAGMA,
    header::COOKIE,
];
