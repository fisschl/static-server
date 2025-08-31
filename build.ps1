# 定义Docker镜像目标
$target = "open-source-cn-shanghai.cr.volces.com/open/static-server:latest"

# 构建Docker镜像
Write-Host "Building Docker image..."
docker build -t $target .
if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

# 推送Docker镜像
Write-Host "Pushing Docker image..."
docker push $target
if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker push failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "Docker build and push completed successfully!" -ForegroundColor Green