import { S3Client, GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

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

      const key = pathname.replace(/[/\\]+/g, "/").replace(/^\//, "");

      // 创建GetObjectCommand，设置S3返回的响应头中的缓存控制字段
      const command = new GetObjectCommand({
        Bucket: S3_BUCKET!,
        Key: key,
        ResponseCacheControl: `max-age=${30 * 24 * 60 * 60}`, // 30天缓存
      });

      // 生成预签名URL，有效期设为6天
      const signedUrl = await getSignedUrl(s3Client, command, {
        expiresIn: 6 * 24 * 60 * 60,
      });

      // 重定向到预签名URL，并设置客户端缓存时间为5天
      return new Response(null, {
        status: 302,
        headers: {
          Location: signedUrl,
          "Cache-Control": `max-age=${5 * 24 * 60 * 60}`,
        },
      });
    },
  },
});
