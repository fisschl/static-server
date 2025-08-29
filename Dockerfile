FROM open-source-cn-shanghai.cr.volces.com/open/bun:1.2 AS base
WORKDIR /app

COPY package.json bun.lock ./
RUN bun install --production

FROM base AS builder
COPY . .
RUN bun run build

FROM open-source-cn-shanghai.cr.volces.com/open/bun:1.2 AS production
WORKDIR /app
COPY --from=builder /app/dist ./dist
EXPOSE 3000
CMD ["bun", "./dist/index.js"]
