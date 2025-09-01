FROM open-source-cn-shanghai.cr.volces.com/open/rust:1 AS chef
WORKDIR /app
RUN cargo install cargo-chef

# Planner 阶段：分析项目依赖
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder 阶段：构建依赖和项目
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# 构建依赖（这将被缓存）
RUN cargo chef cook --release --recipe-path recipe.json

# 复制源代码
COPY . .

# 构建项目
RUN cargo build --release

# 运行阶段：使用标准 Debian 12 镜像
FROM open-source-cn-shanghai.cr.volces.com/open/rust:1

# 安装必要的运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    tzdata

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/static-server ./static-server

# 暴露端口
EXPOSE 3000

# 设置启动命令
CMD ["./static-server"]
