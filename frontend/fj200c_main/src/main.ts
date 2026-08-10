/**
 * 发动机测控应用（fj200c_main）入口文件
 *
 * 职责：
 * 1. 创建 Vue 3 应用实例并挂载到 #app
 * 2. 注册全局插件：Pinia、Vue Router、Element Plus（中文）
 * 3. 加载全局样式（含仪表盘深浅主题 theme.css）
 */
import { createApp } from "vue";
import { createPinia } from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";
import "element-plus/es/components/message-box/style/css";

import App from "./App.vue";
import router from "./router";

import "./style.css";
import "@/fj200c_main/styles/theme.css";

/** 创建 Vue 应用实例 */
const app = createApp(App);

/** 注册插件 */
app.use(createPinia());
app.use(router);

/** 挂载到 DOM */
app.mount("#app");
