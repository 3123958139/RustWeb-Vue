/**
 * 设备台账应用（fw150）入口文件
 *
 * 创建 Vue 3 应用实例并注册插件：
 * - Pinia（状态管理）
 * - Vue Router（路由）
 * - Element Plus（UI 组件库，中文语言环境）
 */
import {createApp} from "vue";
import {createPinia} from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";

import App from "./App.vue";
import router from "./router";

import "./style.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");
