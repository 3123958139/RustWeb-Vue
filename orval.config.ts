import { defineConfig } from "orval";

/**
 * RustWeb-Vue API 客户端生成配置。
 *
 * 输入：`openapi/openapi.json`（由 `cargo test export_openapi` 生成）
 * 输出：`packages/shared/src/api/generated/`
 *   - `api/<tag>.ts`：按 OpenAPI tag 拆分的请求函数
 *   - `model/*.ts`：共享类型定义
 *
 * 生成前先运行 `npm run gen:api`（自动执行后端导出 + orval）。
 */
export default defineConfig({
  rustweb: {
    input: {
      target: "./openapi/openapi.json",
    },
    output: {
      workspace: "./packages/shared/src/api/generated",
      target: "./api",
      schemas: "./model",
      client: "axios",
      // v8 起默认 httpClient 为 fetch，显式指定 axios
      httpClient: "axios",
      mode: "tags-split",
      clean: true,
      prettier: true,
      override: {
        mutator: {
          // 相对 workspace（packages/shared/src/api/generated）解析
          path: "../custom-instance.ts",
          name: "customInstance",
        },
      },
    },
  },
});
