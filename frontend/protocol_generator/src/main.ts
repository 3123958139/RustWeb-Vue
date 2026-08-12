/**
 * 通信协议生成应用（protocol_generator）入口文件
 *
 * 创建 Vue 3 应用实例并注册插件：
 * - Pinia（状态管理）
 * - Vue Router（路由）
 * - Element Plus（UI 组件库，中文语言环境）
 * - vue-plugin-hiprint（协议报表打印，挂载到 window.hiprint）
 */
import {createApp} from "vue";
import {createPinia} from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";
import { hiPrintPlugin } from "vue-plugin-hiprint";
import "vue-plugin-hiprint/dist/print-lock.css";

import App from "./App.vue";
import router from "./router";

import "./style.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(hiPrintPlugin, "$hiprint");

app.mount("#app");