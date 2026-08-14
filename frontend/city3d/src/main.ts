/**
 * @module main
 * @description 前端应用（city3d）入口文件
 *
 * 职责：创建 Vue 应用、注册 Pinia 与路由、挂载到 `#app`。
 * 暗色主题 CSS 与 Element Plus Message 样式在此全局引入。
 */
import { createApp } from "vue";
import { createPinia } from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";

import App from "./App.vue";
import router from "./router";

import "./style.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");