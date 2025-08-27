import { S3Client, GetObjectCommand } from "@aws-sdk/client-s3";
import type { GetObjectCommandInput } from "@aws-sdk/client-s3";

export const createS3Client = () => {
  const { S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY } = Bun.env;
  return new S3Client({
    region: "tos-s3-cn-shanghai",
    credentials: {
      accessKeyId: S3_ACCESS_KEY_ID!,
      secretAccessKey: S3_SECRET_ACCESS_KEY!,
    },
    endpoint: "https://tos-s3-cn-shanghai.volces.com",
    forcePathStyle: false,
  });
};

const s3Client = createS3Client();

Bun.serve({
  routes: {
    "/static/*": async (req: Request) => {
      const { pathname } = new URL(req.url);
      const { S3_BUCKET } = Bun.env;

      const key = pathname.replace(/[/\\]+/g, "/").replace(/^\/+/, "");

      // 获取Range头信息
      const range = req.headers.get("range");

      // 构建GetObjectCommand参数
      const params: GetObjectCommandInput = {
        Bucket: S3_BUCKET!,
        Key: key,
      };

      // 如果有Range参数，则添加到请求中
      if (range) params.Range = range;

      // 创建GetObjectCommand
      const command = new GetObjectCommand(params);

      // 直接从S3获取对象
      const response = await s3Client.send(command);

      // 使用Headers对象构建响应头
      const headers = new Headers();
      headers.set("Cache-Control", `public, max-age=${30 * 24 * 60 * 60}`); // 30天客户端缓存

      // 添加S3返回的元数据到响应头
      if (response.ContentType)
        headers.set("Content-Type", response.ContentType);
      if (response.ContentLength)
        headers.set("Content-Length", response.ContentLength.toString());
      if (response.ETag) headers.set("ETag", response.ETag);
      if (response.LastModified)
        headers.set("Last-Modified", response.LastModified.toUTCString());
      if (range && response.ContentRange)
        headers.set("Content-Range", response.ContentRange);

      const { httpStatusCode } = response.$metadata;
      return new Response(response.Body, {
        status: httpStatusCode,
        headers,
      });
    },
  },
});
