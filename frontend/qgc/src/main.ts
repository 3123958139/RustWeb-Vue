/**
 * @module main
 * @description 飞控地面站应用（qgc）入口文件
 *
 * 创建 Vue 3 应用实例并注册插件：
 * - Pinia（状态管理）
 * - Vue Router（路由）
 * - Element Plus（UI 组件库，暗色主题）
 */
import { createApp } from "vue";
import { createPinia } from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";

import App from "./App.vue";
import router from "./router";

import "@shared/style.css";
import "@/qgc/qgc.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");
