# static-server

一个使用Rust和Axum构建的高性能静态文件服务器，能够从S3兼容存储桶提供文件服务，支持SPA（单页应用）路由和智能缓存。

## 功能特点

- **S3存储桶支持**: 从S3兼容存储桶提供静态文件服务
- **SPA路由支持**: 自动回退到index.html以支持单页应用路由
- **智能缓存**: 基于文件扩展名的智能缓存控制
- **预签名URL**: 使用S3预签名URL确保安全访问
- **高性能缓存**: 使用moka缓存减少重复S3请求
- **CORS支持**: 完整的跨域资源共享支持
- **请求转发**: 支持Range请求、条件请求等高级HTTP功能
- **Docker部署**: 完整的Docker容器化支持
- **多阶段构建**: 优化的Docker多阶段构建减少镜像大小

## 技术栈

- **框架**: Axum (基于Tokio的异步Web框架)
- **S3客户端**: AWS SDK for Rust
- **缓存**: Moka高性能缓存库
- **HTTP客户端**: Reqwest
- **日志**: Tracing + Tracing Subscriber
- **配置**: dotenv环境变量管理

## 环境变量配置

运行前需要设置以下环境变量：

```bash
# AWS S3配置（支持所有S3兼容服务）
AWS_ACCESS_KEY_ID=your_access_key_id
AWS_SECRET_ACCESS_KEY=your_secret_access_key
AWS_REGION=your_region  # 例如：us-east-1, cn-hangzhou
AWS_ENDPOINT_URL=your_s3_endpoint  # 例如：https://oss-cn-hangzhou.aliyuncs.com

# 必需配置
S3_BUCKET=your_bucket_name  # S3存储桶名称

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
S3_BUCKET=your-bucket-name
PORT=3000
```

## Docker部署

### 构建Docker镜像
```bash
# 使用默认构建脚本（Windows PowerShell）
./build.ps1

# 或手动构建
docker build -t static-server .
```

### 运行Docker容器
```bash
docker run -d -p 3000:3000 \
  -e AWS_ACCESS_KEY_ID=your_access_key_id \
  -e AWS_SECRET_ACCESS_KEY=your_secret_access_key \
  -e AWS_REGION=your_region \
  -e AWS_ENDPOINT_URL=your_s3_endpoint \
  -e S3_BUCKET=your_bucket_name \
  static-server
```

### 使用Docker Compose
创建 `docker-compose.yml`：
```yaml
version: '3.8'
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
      - S3_BUCKET=your_bucket_name
    restart: unless-stopped
```

## 缓存策略

服务器实现了智能缓存策略：

- **缓存文件**: CSS、JS、图片、字体等静态资源（30天缓存）
- **不缓存文件**: HTML、HTM文件（避免SPA路由问题）
- **内存缓存**: 路径查找结果缓存60秒，减少S3 API调用

## SPA支持

服务器支持单页应用路由，当请求的文件不存在时：
1. 首先检查请求路径对应的文件
2. 如果不存在，查找第一级目录下的index.html
3. 返回index.html内容，由前端路由处理

## 性能特性

- **异步处理**: 基于Tokio的完全异步架构
- **流式传输**: 支持大文件流式传输，减少内存使用
- **连接池**: Reqwest客户端连接池优化
- **缓存优化**: 多级缓存减少S3 API调用

## 支持的S3服务

支持所有S3兼容的云存储服务：
- AWS S3
- 阿里云OSS
- 腾讯云COS
- 火山引擎TOS
- MinIO
- 其他S3兼容服务

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
