import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(defineAppConfig({ app: "protocol_generator", port: 5180 }, __dirname));