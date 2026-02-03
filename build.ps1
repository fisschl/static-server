# 自动创建 dist 目录（如果不存在）
New-Item -ItemType Directory -Force -Path ./dist | Out-Null

docker build -t static-server .
$container = docker create static-server
docker cp "${container}:/root/static-server" ./dist/static-server
docker rm $container
