import { createRouter, createWebHistory } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { Permission } from "@/types";

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
      meta: {
        requiresAuth: true,
        permissions: [Permission.Dashboard],
      },
    },
    {
      path: "/posts",
      name: "Posts",
      component: () => import("@/views/Posts.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.PostsRead],
      },
    },
    {
      path: "/posts/create",
      name: "CreatePost",
      component: () => import("@/views/CreatePost.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.PostsWrite],
      },
    },
    {
      path: "/posts/:id",
      name: "PostDetail",
      component: () => import("@/views/PostDetail.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.PostsRead],
      },
    },
    {
      path: "/posts/:id/edit",
      name: "EditPost",
      component: () => import("@/views/EditPost.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.PostsWrite],
      },
    },
    {
      path: "/users",
      name: "Users",
      component: () => import("@/views/Users.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.UsersRead],
      },
    },
    {
      path: "/users/create",
      name: "CreateUser",
      component: () => import("@/views/CreateUser.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.UsersWrite],
      },
    },
    {
      path: "/profile",
      name: "Profile",
      component: () => import("@/views/Profile.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Settings],
      },
    },
    {
      path: "/settings",
      name: "Settings",
      component: () => import("@/views/Settings.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.Settings],
      },
    },
    {
      path: "/menu-management",
      name: "MenuManagement",
      component: () => import("@/views/MenuManagement.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.SystemAdmin],
      },
    },
    {
      path: "/permission-management",
      name: "PermissionManagement",
      component: () => import("@/views/PermissionManagement.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.SystemAdmin],
      },
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

  // 检查认证要求
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    next("/login");
    return;
  }

  // 检查访客要求
  if (to.meta.requiresGuest && authStore.isAuthenticated) {
    next("/dashboard");
    return;
  }

  // 检查权限要求
  if (to.meta.permissions) {
    const requiredPermissions = to.meta.permissions as Permission[];
    const hasPermission = requiredPermissions.some((permission) => authStore.hasPermission(permission));

    if (!hasPermission) {
      // 权限不足，重定向到仪表盘或显示错误页面
      next("/dashboard");
      return;
    }
  }

  next();
});

export default router;
