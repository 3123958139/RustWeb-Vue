/**
 * 閫氫俊鍗忚鐢熸垚搴旂敤锛坧rotocol_generator锛夊叆鍙ｆ枃浠?
 *
 * 鍒涘缓 Vue 3 搴旂敤瀹炰緥骞舵敞鍐屾彃浠讹細
 * - Pinia锛堢姸鎬佺鐞嗭級
 * - Vue Router锛堣矾鐢憋級
 * - Element Plus锛圲I 缁勪欢搴擄紝涓枃璇█鐜锛?
 * - vue-plugin-hiprint锛堝崗璁姤琛ㄦ墦鍗帮紝鎸傝浇鍒?window.hiprint锛?
 */
import {createApp} from "vue";
import {createPinia} from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";
import { hiPrintPlugin } from "vue-plugin-hiprint";
import "vue-plugin-hiprint/dist/print-lock.css";

import App from "./App.vue";
import router from "./router";

import "@shared/style.css";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(hiPrintPlugin, "$hiprint");

app.mount("#app");