/**
 * @module vite.config
 * @description qgc 应用构建配置（Vite）
 *
 * 一切公共配置由 `defineAppConfig`（`build/vite.base.ts`）工厂提供，本文件只声明应用标识。
 * dev 端口 5181、`ws: true`（需要 WebSocket 的应用打开该开关）。
 * `@`/`@shared` alias、`/api` 代理、`base` 路径均由工厂统一处理。
 *
 * 3D 地图（Cesium）：不依赖 vite-plugin-cesium，Cesium 直接参与打包
 * （`build/vite.base.ts` manualChunks 单独拆出 cesium chunk），只需运行时静态资源：
 * - dev：`configureServer` 中间件把 `node_modules/cesium/Build/CesiumUnminified`
 *   托管到 `/cesium/`（Workers/Assets/ThirdParty，供运行时按 CESIUM_BASE_URL 加载）
 * - build：`closeBundle` 把 Assets/ThirdParty/Workers 拷贝进 `dist/cesium/`
 *   （`/qgc` 前缀由后端静态托管映射，故 dist 内位于顶层）
 * - `CESIUM_BASE_URL` 全局变量在页面代码中按 `import.meta.env.BASE_URL` 设置
 */
import { cpSync, createReadStream, statSync } from "node:fs";
import type { ServerResponse } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, mergeConfig, type Connect, type Plugin } from "vite";
import { defineAppConfig } from "../../build/vite.base";

/** Cesium 构建产物目录（workspaces 提升到根 node_modules，回退两级） */
const CESIUM_BUILD = fileURLToPath(new URL("../../node_modules/cesium/Build", import.meta.url));

/** 常见 MIME 映射（Cesium 运行时资源：Workers js / Assets 图片 / ThirdParty） */
const MIME: Record<string, string> = {
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".css": "text/css",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".wasm": "application/wasm",
  ".ktx2": "image/ktx2",
};

/** 零依赖静态文件中间件（dev 托管 Cesium 运行时资源） */
function staticFiles(root: string) {
  return (req: Connect.IncomingMessage, res: ServerResponse, next: Connect.NextFunction) => {
    const rel = decodeURIComponent((req.url ?? "").split("?")[0]).replace(/^\/+/, "");
    const file = path.resolve(root, rel);
    if (!file.startsWith(path.resolve(root))) {
      res.statusCode = 403;
      res.end();
      return;
    }
    let isFile = false;
    try {
      isFile = statSync(file).isFile();
    } catch {
      // 不存在则交给后续中间件
    }
    if (!isFile) {
      next();
      return;
    }
    res.setHeader("Content-Type", MIME[path.extname(file).toLowerCase()] ?? "application/octet-stream");
    res.setHeader("Cache-Control", "no-cache");
    createReadStream(file).pipe(res);
  };
}

/**
 * Cesium 集成插件：
 * - dev：静态托管 Cesium 运行时资源（CesiumUnminified 便于调试）
 * - build：拷贝运行时资源到 dist/cesium/
 */
function cesiumIntegration(): Plugin {
  return {
    name: "cesium-integration",
    configureServer(server) {
      server.middlewares.use("/cesium", staticFiles(path.join(CESIUM_BUILD, "CesiumUnminified")));
    },
    async closeBundle() {
      const src = path.join(CESIUM_BUILD, "Cesium");
      const dst = fileURLToPath(new URL("./dist/cesium", import.meta.url));
      for (const dir of ["Assets", "ThirdParty", "Workers"]) {
        cpSync(path.join(src, dir), path.join(dst, dir), { recursive: true });
      }
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig(
  mergeConfig(
    defineAppConfig({ app: "qgc", port: 5181, ws: true }, __dirname),
    {
      plugins: [cesiumIntegration()],
      build: {
        // Cesium 打包后单个 chunk 较大，放宽告警阈值
        chunkSizeWarningLimit: 5000,
      },
    },
  ),
);