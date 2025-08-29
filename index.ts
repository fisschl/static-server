import { S3Client } from "bun";
import { extname } from "path";

// 定义共享的 CORS headers
const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
  "Access-Control-Allow-Headers": "Range, Content-Type, Authorization",
};

const { S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY, S3_ENDPOINT } = Bun.env;

const s3 = new S3Client({
  accessKeyId: S3_ACCESS_KEY_ID!,
  secretAccessKey: S3_SECRET_ACCESS_KEY!,
  endpoint: S3_ENDPOINT!,
  virtualHostedStyle: true,
});

const findExistsKey = async (pathname: string): Promise<string | undefined> => {
  const list = pathname.split(/[/\\]+/g).filter(Boolean);
  if (!list.length) {
    const key = "index.html";
    const isExist = await s3.exists(key);
    if (isExist) return key;
    else return undefined;
  }
  const baseKey = list.join("/");
  const isExist = await s3.exists(baseKey);
  if (isExist) return baseKey;
  while (list.length) {
    const key = [...list, "index.html"].join("/");
    const isExist = await s3.exists(key);
    if (isExist) return key;
    list.pop();
  }
  return findExistsKey("");
};

const NoCacheExts = [".html", ".htm"];

const pick = (headers: Headers, items: string[]) => {
  const result: Record<string, string> = {};
  items.forEach((key) => {
    const value = headers.get(key);
    if (value) result[key] = value;
  });
  return result;
};

Bun.serve({
  async fetch(req) {
    // 处理 CORS 预检请求
    if (req.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: CORS_HEADERS,
      });
    }

    const { pathname } = new URL(req.url);
    const fileKey = await findExistsKey(pathname);

    if (!fileKey) {
      return new Response("Not Found", {
        status: 404,
        headers: CORS_HEADERS,
      });
    }

    const response = await fetch(s3.presign(fileKey), {
      headers: req.headers,
    });

    const headers: Record<string, string> = {
      ...CORS_HEADERS,
      ...pick(response.headers, [
        "Content-Type",
        "ETag",
        "Last-Modified",
        "Content-Range",
        "Content-Length",
        "Content-Encoding",
        "Accept-Ranges",
        "Vary",
        "Expires"
      ]),
    };

    const fileExt = extname(fileKey);
    if (!NoCacheExts.includes(fileExt) && !headers["Cache-Control"]) {
      headers["Cache-Control"] = `public, max-age=${30 * 24 * 60 * 60}`;
    }

    return new Response(response.body, {
      status: response.status,
      headers,
    });
  },
  port: 3000,
});
