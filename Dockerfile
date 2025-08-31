# 使用官方Rust镜像作为构建阶段
FROM rust:1.78-slim-buster AS builder

# 设置工作目录
WORKDIR /app

# 复制Cargo.toml和Cargo.lock文件
COPY Cargo.toml Cargo.lock ./

# 创建一个占位符main.rs文件以构建依赖
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# 构建依赖（这将缓存依赖层）
RUN cargo build --release

# 复制真实的源代码
COPY src ./src

# 删除占位符main.rs
RUN rm src/main.rs

# 复制真实的main.rs
COPY src/main.rs ./src/

# 重新构建项目
RUN cargo build --release

# 使用Alpine作为运行阶段基础镜像
FROM alpine:3.18

# 安装必要的依赖
RUN apk --no-cache add ca-certificates

# 创建非root用户运行应用
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/static-server /app/

# 更改文件所有权
RUN chown -R appuser:appgroup /app

# 切换到非root用户
USER appuser

# 暴露端口
EXPOSE 3000

# 设置启动命令
CMD ["./static-server"]
