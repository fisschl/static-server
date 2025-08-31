# static-server

一个使用Rust和Actix Web构建的静态文件服务器，能够从S3存储桶提供文件服务。

## 功能特点
- 从S3兼容存储桶提供静态文件服务
- 支持自动索引页查找（index.html）
- 实现CORS跨域资源共享
- 支持缓存控制
- 提供Docker部署支持

## 环境变量配置
运行前需要设置以下环境变量：

```bash
# AWS S3配置
S3_ACCESS_KEY_ID=your_access_key
S3_SECRET_ACCESS_KEY=your_secret_key
S3_ENDPOINT=your_s3_endpoint
S3_BUCKET=your_bucket_name

# 可选配置
PORT=3000  # 默认端口
```

## 本地开发

### 安装依赖
```bash
cargo build
```

### 运行项目
```bash
# 开发模式
cargo run

# 生产模式
cargo run --release
```

## Docker部署

### 使用PowerShell脚本构建和推送镜像
```powershell
# Windows PowerShell
.uild.ps1
```

### 手动构建Docker镜像
```bash
docker build -t static-server .
```

### 运行Docker容器
```bash
docker run -d -p 3000:3000 \
  -e S3_ACCESS_KEY_ID=your_access_key \
  -e S3_SECRET_ACCESS_KEY=your_secret_key \
  -e S3_ENDPOINT=your_s3_endpoint \
  -e S3_BUCKET=your_bucket_name \
  static-server
```

## 项目结构
```
├── .dockerignore       # Docker忽略文件
├── .gitignore          # Git忽略文件
├── Cargo.toml          # Rust项目配置
├── Dockerfile          # Docker构建文件
├── LICENSE             # 许可证文件
├── README.md           # 项目说明文档
├── build.ps1           # PowerShell构建脚本
└── src/                # 源代码目录
    └── main.rs         # 主程序文件
```
