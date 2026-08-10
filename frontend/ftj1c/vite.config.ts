import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "ftj1c", port: 5176, ws: true }, __dirname));
