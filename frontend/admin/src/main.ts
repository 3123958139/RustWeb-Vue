/**
 * 绠＄悊鍚庡彴鍏ュ彛鏂囦欢
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
import "element-plus/es/components/message-box/style/css";

import App from "./App.vue";
import router from "./router";

import "@shared/style.css";

/** 鍒涘缓 Vue 搴旂敤瀹炰緥锛屼紶鍏ユ牴缁勪欢 */
const app = createApp(App);

/** 娉ㄥ唽鎻掍欢锛歅inia 鎻愪緵鍝嶅簲寮忕姸鎬佺鐞嗭紝Router 鎻愪緵璺敱瀵艰埅锛孍lement Plus 鎻愪緵 UI 缁勪欢 */
app.use(createPinia());
app.use(router);

/** 灏嗗簲鐢ㄦ寕杞藉埌 DOM 涓?id 涓?app 鐨勫厓绱犱笂 */
app.mount("#app");
