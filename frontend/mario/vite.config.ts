/**
 * @module vite.config
 * @description mario 应用构建配置（Vite）
 */
import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "mario", port: 5182 }, __dirname));