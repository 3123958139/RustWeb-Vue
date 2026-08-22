/**
 * @module router
 * @description 前端应用（mario）路由配置
 */
import { useAuthStore } from "@/stores/auth";
import { Permission, createAppRouter } from "@shared";

const router = createAppRouter({
  app: "mario",
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
      path: "/mario",
      redirect: "/mario/main",
    },
    {
      path: "/mario/main",
      name: "Game",
      component: () => import("@/mario/views/GameView.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.MarioMonitor],
      },
    },
    {
      path: "/mario/rank",
      name: "Rank",
      component: () => import("@/mario/views/RankView.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.MarioMonitor],
      },
    },
    {
      path: "/:pathMatch(.*)*",
      redirect: "/login",
    },
  ],
});

export default router;