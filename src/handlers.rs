//! HTTP请求处理模块
//!
//! 此模块包含了处理不同类型HTTP请求的所有处理器：
//! - 文件服务处理器
//! - 代理处理器
//! - SPA键查找处理器

pub mod compatible_mode;
pub mod constants;
pub mod files;
pub mod proxy;
pub mod spa_key;

// 重新导出主要的公共接口
pub use compatible_mode::handle_compatible_mode_proxy;
pub use files::handle_files;
