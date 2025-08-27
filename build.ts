import { $ } from "bun";

const target = "open-source-cn-shanghai.cr.volces.com/open/static-server:latest";

await $`docker build -t ${target} .`;
await $`docker push ${target}`;
