#!/bin/bash
set -e

# 自动创建 dist 目录（如果不存在）
mkdir -p ./dist

docker build -t static-server .
container=$(docker create static-server)
docker cp "${container}:/root/static-server" ./dist/static-server
docker rm "$container"

# 上传构建产物到 rclone
rclone copyto ./dist/static-server tos:muelsyse/static-server/static-server
