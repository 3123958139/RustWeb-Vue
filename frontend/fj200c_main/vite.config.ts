import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(
  defineAppConfig({ app: "fj200c_main", port: 5179, ws: true }, __dirname),
);
