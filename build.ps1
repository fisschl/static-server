# 设置全局错误处理偏好，命令出错时自动停止执行
$ErrorActionPreference = "Stop"

# 定义Docker镜像目标
$target = "open-source-cn-shanghai.cr.volces.com/open/static-server:latest"

# 构建Docker镜像
Write-Host "Building Docker image..."
docker build -t $target .

# 推送Docker镜像
Write-Host "Pushing Docker image..."
docker push $target

Write-Host "Docker build and push completed successfully!" -ForegroundColor Green
