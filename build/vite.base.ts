import path from "node:path";
import vue from "@vitejs/plugin-vue";
import Components from "unplugin-vue-components/vite";
import { ElementPlusResolver } from "unplugin-vue-components/resolvers";
import type { UserConfig } from "vite";

export interface AppConfig {
  app: string;
  port: number;
  ws?: boolean;
}

/**
 * 7 个前端共享的 Vite 配置工厂（docs/optimization-plan.md P2「vite 共享工厂」）。
 * 注意：appDir 必须显式传入（各应用 vite.config.ts 传 __dirname），
 * 因工厂内 __dirname 实测指向 build/ 而非应用目录。
 */
export function defineAppConfig(opts: AppConfig, appDir: string): UserConfig {
  const isBuild = process.env.NODE_ENV === "production" || process.argv.includes("build");

  return {
    plugins: [
      vue(),
      Components({
        dts: path.join(appDir, "components.d.ts"),
        resolvers: [ElementPlusResolver({ importStyle: "css" })],
      }),
    ],
    resolve: {
      alias: {
        "@": path.join(appDir, "src"),
        "@shared": path.join(appDir, "../../packages/shared/src"),
      },
      dedupe: ["vue", "pinia", "element-plus"],
    },
    server: {
      port: opts.port,
      strictPort: true,
      proxy: {
        "/api": {
          target: "http://localhost:3000",
          changeOrigin: true,
          ws: opts.ws ?? false,
        },
      },
    },
    base: isBuild ? `/${opts.app}/` : "/",
    build: {
      chunkSizeWarningLimit: 1500,
      rollupOptions: {
        output: {
          manualChunks(id: string) {
            if (!id.includes("node_modules")) return undefined;
            if (id.includes("element-plus") || id.includes("@element-plus")) return "element-plus";
            if (id.includes("echarts")) return "echarts";
            if (id.includes("cesium")) return "cesium";
            if (
              id.includes("/vue/") ||
              id.includes("vue-router") ||
              id.includes("pinia") ||
              id.includes("@vue/") ||
              id.includes("@vueuse")
            ) {
              return "vue-vendor";
            }
            return "vendor";
          },
        },
      },
    },
  };
}
