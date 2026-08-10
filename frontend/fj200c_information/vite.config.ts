import { defineConfig } from "vite";
import { defineAppConfig } from "../../build/vite.base";

// https://vitejs.dev/config/
export default defineConfig(
  defineAppConfig({ app: "fj200c_information", port: 5173, ws: true }, __dirname),
);
