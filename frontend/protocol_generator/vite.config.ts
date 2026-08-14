/**
 * @module vite.config
 * @description protocol_generator 应用构建配置（Vite）
 *
* 一行工厂调用 `defineAppConfig`（见 `build/vite.base.ts`），传入应用名、
 * dev 端口与 `ws` 代理标志（需要 WebSocket 的应用按需开启），
 * 共享 `@`/`@shared` alias、`/api` 代理与 `base` 路径。
 */
import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "protocol_generator", port: 5180 }, __dirname));