# static-server

一个使用 Rust 和 Axum 构建的高性能静态文件服务器，能够从 S3 兼容存储桶提供文件服务，支持 SPA（单页应用）路由和智能缓存。

## 功能特点

- **S3 存储桶支持**: 从 S3 兼容存储桶提供静态文件服务
- **SPA 路由支持**: 自动回退到 index.html 以支持单页应用路由
- **智能缓存**: 基于文件扩展名的智能缓存控制
- **预签名 URL**: 使用 S3 预签名 URL 确保安全访问
- **CORS 支持**: 完整的跨域资源共享支持
- **请求转发**: 支持 Range 请求、条件请求等高级 HTTP 功能
- **Docker 部署**: 完整的 Docker 容器化支持
- **多阶段构建**: 优化的 Docker 多阶段构建减少镜像大小

## 技术栈

- **框架**: Axum (基于 Tokio 的异步 Web 框架)
- **S3 客户端**: AWS SDK for Rust
- **缓存**: cached 宏缓存库
- **HTTP 客户端**: Reqwest
- **日志**: Tracing + Tracing Subscriber
- **配置**: dotenv 环境变量管理

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

# 可选配置
PORT=3000  # 服务器监听端口，默认3000
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

### 环境变量设置

创建 `.env` 文件或在命令行中设置环境变量：

```bash
# .env 文件示例
AWS_ACCESS_KEY_ID=your_access_key_id
AWS_SECRET_ACCESS_KEY=your_secret_access_key
AWS_REGION=cn-hangzhou
AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
AWS_BUCKET=your-bucket-name
PORT=3000
```

## Docker 部署

### 构建 Docker 镜像

```bash
# 使用默认构建脚本（Windows PowerShell）
./build.ps1

# 或手动构建
docker build -t static-server .
```

### 运行 Docker 容器

```bash
docker run -d -p 3000:3000 \
  -e AWS_ACCESS_KEY_ID=your_access_key_id \
  -e AWS_SECRET_ACCESS_KEY=your_secret_access_key \
  -e AWS_REGION=your_region \
  -e AWS_ENDPOINT_URL=your_s3_endpoint \
  -e AWS_BUCKET=your_bucket_name \
  static-server
```

### 使用 Docker Compose

创建 `docker-compose.yml`：

```yaml
version: "3.8"
services:
  static-server:
    image: static-server
    ports:
      - "3000:3000"
    environment:
      - AWS_ACCESS_KEY_ID=your_access_key_id
      - AWS_SECRET_ACCESS_KEY=your_secret_access_key
      - AWS_REGION=your_region
      - AWS_ENDPOINT_URL=your_s3_endpoint
      - AWS_BUCKET=your_bucket_name
    restart: unless-stopped
```

## 缓存策略

服务器实现了智能缓存策略：

- **缓存文件**: CSS、JS、图片、字体等静态资源（30 天缓存）
- **不缓存文件**: HTML、HTM 文件（避免 SPA 路由问题）
- **内存缓存**: 路径查找结果缓存 60 秒，减少 S3 API 调用

## SPA 支持

服务器支持单页应用路由，当请求的文件不存在时：

1. 首先检查请求路径对应的文件
2. 如果不存在，查找第一级目录下的 index.html
3. 返回 index.html 内容，由前端路由处理

## 性能特性

- **异步处理**: 基于 Tokio 的完全异步架构
- **流式传输**: 支持大文件流式传输，减少内存使用
- **连接池**: Reqwest 客户端连接池优化
- **缓存优化**: 多级缓存减少 S3 API 调用

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
├── main.rs          # 应用入口点
├── lib.rs           # 应用配置和路由
├── handlers/        # 请求处理器
│   ├── files.rs     # 文件处理逻辑
│   └── spa_key.rs   # SPA路由支持
└── s3/             # S3相关功能
    ├── config.rs    # S3配置
    └── presign.rs   # 预签名URL生成
```

## 许可证

MIT License - 详见 LICENSE 文件
