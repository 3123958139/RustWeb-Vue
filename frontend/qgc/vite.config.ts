/**
 * @module vite.config
 * @description qgc 应用构建配置（Vite）
 *
 * 一切公共配置由 `defineAppConfig`（`build/vite.base.ts`）工厂提供，本文件只声明应用标识。
 * dev 端口 5181、`ws: true`（需要 WebSocket 的应用打开该开关）。
 * `@`/`@shared` alias、`/api` 代理、`base` 路径均由工厂统一处理。
 */
import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "qgc", port: 5181, ws: true }, __dirname));
