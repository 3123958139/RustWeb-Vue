/**
 * 发动机监控应用（fj200c_information）入口文件
 *
 * 职责：
 * 1. 创建 Vue 3 应用实例并挂载到 #app
 * 2. 注册全局插件：Pinia（状态管理）、Vue Router（路由）、Element Plus（UI 组件库）
 * 3. 设置 Element Plus 为中文语言环境
 */
import { createApp } from "vue";
import { createPinia } from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";

import App from "./App.vue";
import router from "./router";

import "./style.css";

/** 创建 Vue 应用实例 */
const app = createApp(App);

/** 注册插件 */
app.use(createPinia());
app.use(router);

/** 挂载到 DOM */
app.mount("#app");
