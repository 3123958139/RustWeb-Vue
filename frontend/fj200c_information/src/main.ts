/**
 * 鍙戝姩鏈虹洃鎺у簲鐢紙fj200c_information锛夊叆鍙ｆ枃浠?
 *
 * 鑱岃矗锛?
 * 1. 鍒涘缓 Vue 3 搴旂敤瀹炰緥骞舵寕杞藉埌 #app
 * 2. 娉ㄥ唽鍏ㄥ眬鎻掍欢锛歅inia锛堢姸鎬佺鐞嗭級銆乂ue Router锛堣矾鐢憋級銆丒lement Plus锛圲I 缁勪欢搴擄級
 * 3. 璁剧疆 Element Plus 涓轰腑鏂囪瑷€鐜
 */
import { createApp } from "vue";
import { createPinia } from "pinia";
import "element-plus/theme-chalk/dark/css-vars.css";
import "element-plus/es/components/message/style/css";

import App from "./App.vue";
import router from "./router";

import "@shared/style.css";

/** 鍒涘缓 Vue 搴旂敤瀹炰緥 */
const app = createApp(App);

/** 娉ㄥ唽鎻掍欢 */
app.use(createPinia());
app.use(router);

/** 鎸傝浇鍒?DOM */
app.mount("#app");
