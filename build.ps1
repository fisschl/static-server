$ErrorActionPreference = "Stop"

# 定义Docker镜像目标
$target = "static-server"

# 构建Docker镜像
docker build -t $target .

# 创建临时容器来提取构建产物
$containerId = docker create $target

# 导出构建产物到目标目录
New-Item -ItemType Directory -Force -Path "./dist"
docker cp "${containerId}:/root/static-server" "./dist/static-server"

# 删除临时容器
docker rm $containerId
