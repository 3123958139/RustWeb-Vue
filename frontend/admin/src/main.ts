/**
 * 管理后台入口文件
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
import "element-plus/es/components/message-box/style/css";

import App from "./App.vue";
import router from "./router";

import "./style.css";

/** 创建 Vue 应用实例，传入根组件 */
const app = createApp(App);

/** 注册插件：Pinia 提供响应式状态管理，Router 提供路由导航，Element Plus 提供 UI 组件 */
app.use(createPinia());
app.use(router);

/** 将应用挂载到 DOM 中 id 为 app 的元素上 */
app.mount("#app");
