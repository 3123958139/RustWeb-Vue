/**
 * 发动机监控应用路由配置
 *
 * 路由结构：
 * /                   → 重定向到 /login
 * /login              → 登录页
 * /template           → 模板面板（共享组件）
 * /fj200c_information             → 重定向到 /fj200c_information/monitor
 * /fj200c_information/monitor     → 实时监控页
 * /fj200c_information/visual      → 可视化分析页
 * /fj200c_information/data        → 数据记录页
 * /fj200c_information/config      → 配置文件编辑页
 * /fj200c_information/help        → 帮助页
 *
 * 生产环境路径前缀为 /fj200c_information/，开发环境为 /。
 */
import { useAuthStore } from "@/stores/auth";
import { Permission, createAppRouter } from "@shared";

const router = createAppRouter({
  app: "fj200c_information",
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
      path: "/template",
      name: "TemplatePanel",
      // 引用 @shared 中的共享模板组件
      component: () => import("@shared/template/TemplatePanel.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
      },
    },
    {
      // 兼容旧入口：/fj200c_information 重定向到实时监控页
      path: "/fj200c_information",
      redirect: "/fj200c_information/monitor",
    },
    {
      path: "/fj200c_information/monitor",
      name: "Fj200cInformationMonitor",
      component: () => import("@/fj200c_information/views/Monitor.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
      },
    },
    {
      path: "/fj200c_information/visual",
      name: "Fj200cInformationVisual",
      component: () => import("@/fj200c_information/views/Visual.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
      },
    },
    {
      path: "/fj200c_information/data",
      name: "Fj200cInformationData",
      component: () => import("@/fj200c_information/views/Data.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
      },
    },
    {
      path: "/fj200c_information/config",
      name: "Fj200cInformationConfig",
      component: () => import("@/fj200c_information/views/Config.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
      },
    },
    {
      path: "/fj200c_information/help",
      name: "Fj200cInformationHelp",
      component: () => import("@/fj200c_information/views/Help.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Fj200cInformationMonitor],
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
