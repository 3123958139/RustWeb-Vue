import { createRouter, createWebHistory } from "vue-router";
import { useAuthStore } from "@/stores/auth";

const router = createRouter({
  history: createWebHistory(),
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
      path: "/register",
      name: "Register",
      component: () => import("@/views/Register.vue"),
      meta: { requiresGuest: true },
    },
    {
      path: "/dashboard",
      name: "Dashboard",
      component: () => import("@/views/Dashboard.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/posts",
      name: "Posts",
      component: () => import("@/views/Posts.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/posts/create",
      name: "CreatePost",
      component: () => import("@/views/CreatePost.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/posts/:id",
      name: "PostDetail",
      component: () => import("@/views/PostDetail.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/posts/:id/edit",
      name: "EditPost",
      component: () => import("@/views/EditPost.vue"),
      meta: { requiresAuth: true },
    },
  ],
});

// 路由守卫
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore();

  // 初始化认证状态
  if (!authStore.isAuthenticated) {
    authStore.initAuth();
  }

  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    next("/login");
  } else if (to.meta.requiresGuest && authStore.isAuthenticated) {
    next("/dashboard");
  } else {
    next();
  }
});

export default router;
