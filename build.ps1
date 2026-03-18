#!/usr/bin/env pwsh
# 跨平台构建脚本 (PowerShell)
# 前置要求: Docker, rclone

$ErrorActionPreference = "Stop"

$DistDir = "./dist"
$ImageName = "static-server"
$RemotePath = "tos:muelsyse/static-server/static-server"

# 创建 dist 目录
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# 构建 Docker 镜像
docker build -t $ImageName .

# 创建临时容器并提取构建产物
$container = docker create $ImageName
try {
    docker cp "${container}:/root/static-server" "$DistDir/static-server"
} finally {
    docker rm $container | Out-Null
}

# 上传到远程存储
rclone copyto "$DistDir/static-server" $RemotePath

Write-Host "构建并上传完成: $DistDir/static-server -> $RemotePath"
