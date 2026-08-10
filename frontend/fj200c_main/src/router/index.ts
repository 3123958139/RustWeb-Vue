/**
 * 发动机测控应用（fj200c_main）路由配置
 *
 * 路由结构：
 * /                            → 重定向到 /login
 * /login                       → 登录页
 * /fj200c_main                 → 重定向到 /fj200c_main/monitor
 * /fj200c_main/monitor         → 主仪表盘（1920×1080 scale 缩放）
 * /fj200c_main/experiment-input → 试验信息录入
 * /fj200c_main/experiment-view  → 试验数据查看
 * /fj200c_main/report          → 报表生成（hiprint 打印）
 * /fj200c_main/data            → 数据记录（CSV 浏览）
 * /fj200c_main/config          → 配置文件编辑
 * /fj200c_main/help            → 帮助
 *
 * 生产环境路径前缀为 /fj200c_main/，开发环境为 /。
 */
import { useAuthStore } from "@/stores/auth";
import { Permission, createAppRouter } from "@shared";

const router = createAppRouter({
  app: "fj200c_main",
  useAuthStore,
  routes: [
    {
      path: "/",
      redirect: "/login",
    },
    {
      path: "/login",
      name: "Login",
      component: () => import("@/views/Login.vue"),
      meta: { requiresGuest: true },
    },
    {
      // 兼容旧入口：/fj200c_main 重定向到主仪表盘
      path: "/fj200c_main",
      redirect: "/fj200c_main/monitor",
    },
    {
      path: "/fj200c_main/monitor",
      name: "Fj200cMainMonitor",
      component: () => import("@/fj200c_main/views/Monitor.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/experiment-input",
      name: "Fj200cMainExperimentInput",
      component: () => import("@/fj200c_main/views/ExperimentInput.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/experiment-view",
      name: "Fj200cMainExperimentView",
      component: () => import("@/fj200c_main/views/ExperimentView.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/report",
      name: "Fj200cMainReport",
      component: () => import("@/fj200c_main/views/GenerateReport.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/data",
      name: "Fj200cMainData",
      component: () => import("@/fj200c_main/views/Data.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/config",
      name: "Fj200cMainConfig",
      component: () => import("@/fj200c_main/views/Config.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    {
      path: "/fj200c_main/help",
      name: "Fj200cMainHelp",
      component: () => import("@/fj200c_main/views/Help.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cMainMonitor],
      },
    },
    // 兜底：未匹配路径一律回登录页
    {
      path: "/:pathMatch(.*)*",
      redirect: "/login",
    },
  ],
});

export default router;
