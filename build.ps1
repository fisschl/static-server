docker build -t static-server .
$container = docker create static-server
docker cp "${container}:/root/static-server" ./dist/static-server
docker rm $container
