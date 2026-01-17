# static-server

一个使用 Rust 和 Axum 构建的高性能静态文件服务器和 API 代理服务，能够从 S3 兼容存储桶提供文件服务，同时支持 DeepSeek API 代理，具备 SPA（单页应用）路由、智能缓存和统一请求头过滤等高级功能。

## 功能特点

- **S3 存储桶支持**: 从 S3 兼容存储桶提供静态文件服务
- **API 代理**: DeepSeek API 代理（聊天补全、模型列表、余额查询）
- **统一头部过滤**: 统一的请求头/响应头黑名单管理，支持多场景代理
- **灵活认证**: 应用层认证控制，支持客户端 token 和服务器 token
- **SPA 路由支持**: 自动回退到 index.html 以支持单页应用路由
- **智能缓存**: 基于文件扩展名的智能缓存控制
- **MIME 类型检测**: 自动检测文件类型并设置正确的 Content-Type
- **预签名 URL**: 使用 S3 预签名 URL 确保安全访问
- **CORS 支持**: 完整的跨域资源共享支持
- **流式传输**: 支持大文件和 API 响应的流式传输
- **Docker 部署**: 完整的 Docker 容器化支持
- **多阶段构建**: 优化的 Docker 多阶段构建减少镜像大小

## API 接口

### 静态文件服务

```
GET /{path}
```

从 S3 存储桶提供静态文件服务，支持 SPA 路由回退。

### DeepSeek API 代理

#### 聊天补全
```
POST /free-model/chat/completions
```

代理 DeepSeek 聊天补全 API，支持流式和非流式响应。

**认证**：
- 优先使用客户端提供的 `Authorization` 头
- 如未提供，自动使用服务器配置的 `DEEPSEEK_API_KEY`

#### 模型列表
```
GET /free-model/models
```

查询可用的 DeepSeek 模型列表。

#### 用户余额
```
GET /free-model/user/balance
```

查询 API 账户余额信息。

## 技术栈

- **框架**: Axum (基于 Tokio 的异步 Web 框架)
- **S3 客户端**: AWS SDK for Rust
- **缓存**: cached 宏缓存库
- **HTTP 客户端**: Reqwest
- **MIME 检测**: mime_guess 库用于文件类型识别
- **日志**: Tracing + Tracing Subscriber
- **配置**: dotenvy 环境变量管理

## 环境变量配置

运行前需要设置以下环境变量：

```bash
# AWS S3配置（支持所有S3兼容服务）
AWS_ACCESS_KEY_ID=your_access_key_id
AWS_SECRET_ACCESS_KEY=your_secret_access_key
AWS_REGION=your_region  # 例如：us-east-1, cn-hangzhou
AWS_ENDPOINT_URL=your_s3_endpoint  # 例如：https://oss-cn-hangzhou.aliyuncs.com

# 必需配置
AWS_BUCKET=your_bucket_name  # S3存储桶名称

# DeepSeek API 配置（API 代理功能）
DEEPSEEK_API_KEY=your_deepseek_api_key
```

## 本地开发

### 安装依赖

```bash
cargo build
```

### 运行项目

```bash
# 开发模式（带调试日志）
cargo run

# 生产模式
cargo run --release
```

## Docker 部署

### 构建 Docker 镜像

```bash
# 使用默认构建脚本（Windows PowerShell）
./build.ps1

# 或手动构建
docker build -t static-server .
```

### 运行可执行文件

构建完成后，可执行文件会提取到 `target/static-server`：

```bash
# 直接运行
./target/static-server
```

## 缓存策略

服务器实现了智能缓存策略：

- **缓存文件**: CSS、JS、图片、字体等静态资源（30 天缓存）
- **不缓存文件**: HTML、HTM 文件（避免 SPA 路由问题）
- **内存缓存**: 路径查找结果缓存 60 秒，减少 S3 API 调用

## 请求头过滤

### 统一黑名单策略

项目采用统一的请求头和响应头黑名单，确保所有代理场景的安全性：

#### 请求头黑名单

以下头部会在代理转发时自动移除：

- **连接与传输**: `Host`, `Connection`, `Transfer-Encoding`, `TE`, `Trailer`, `Upgrade`
- **认证与安全**: `Proxy-Authorization`, `Cookie`
- **来源信息**: `Origin`, `Referer`

#### 响应头黑名单

以下头部会在返回给客户端时自动移除：

- **连接与传输**: `Connection`, `TE`, `Trailer`, `Transfer-Encoding`, `Upgrade`
- **CORS 相关**: 所有 `Access-Control-*` 头部（由 CorsLayer 统一管理）
- **Cookie 与缓存**: `Set-Cookie`, `Cache-Control`, `Expires`, `Age`, `Pragma`, `Vary`

### 认证策略

- **DeepSeek API**: 优先使用客户端的 `Authorization` 头，未提供则使用服务器 `DEEPSEEK_API_KEY`
- **S3 文件**: 使用预签名 URL，无需额外认证

## MIME 类型检测

服务器支持自动 MIME 类型检测：

- **智能检测**: 当 S3 响应缺少 Content-Type 时，根据文件扩展名自动猜测
- **广泛支持**: 支持数百种常见文件类型（CSS、JS、PNG、JSON 等）
- **向后兼容**: 保留 S3 原有的 Content-Type，仅在缺失时进行补充
- **浏览器优化**: 确保浏览器能正确处理和渲染各类静态资源

## SPA 支持

服务器支持单页应用路由，当请求的文件不存在时：

1. 首先检查请求路径对应的文件
2. 如果不存在，查找第一级目录下的 index.html
3. 返回 index.html 内容，由前端路由处理

## 性能特性

- **异步处理**: 基于 Tokio 的完全异步架构
- **流式传输**: 支持大文件和 API 响应的流式传输，减少内存使用
- **连接池**: Reqwest 客户端连接池优化
- **缓存优化**: 多级缓存减少 S3 API 调用
- **统一过滤**: 高效的头部过滤逻辑

## 支持的 S3 服务

支持所有 S3 兼容的云存储服务：

- AWS S3
- 阿里云 OSS
- 腾讯云 COS
- 火山引擎 TOS
- MinIO
- 其他 S3 兼容服务

## 开发说明

项目结构：

```
src/
├── main.rs              # 应用入口点
├── lib.rs               # 应用配置和路由
├── handlers.rs          # handlers 模块声明
├── handlers/            # 请求处理器
│   ├── balance.rs       # DeepSeek 余额查询
│   ├── chat_completions.rs  # DeepSeek 聊天补全
│   ├── models.rs        # DeepSeek 模型列表
│   └── files.rs         # S3 文件处理逻辑
├── utils.rs             # utils 模块声明
└── utils/               # 工具函数
    ├── headers.rs       # HTTP 头部过滤和 MIME 检测
    ├── proxy.rs         # 统一代理逻辑和黑名单配置
    ├── s3.rs            # S3 预签名 URL 生成
    └── path.rs          # 路径处理工具
```

## 许可证

MIT License - 详见 LICENSE 文件
