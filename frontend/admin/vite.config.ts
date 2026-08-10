import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "admin", port: 5174 }, __dirname));
