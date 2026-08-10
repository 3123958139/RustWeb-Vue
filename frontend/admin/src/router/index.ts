/**
 * 管理后台路由配置
 *
 * 使用 Vue Router 4 的 HTML5 History 模式。
 * 生产环境路径前缀为 /admin/，开发环境为 /。
 *
 * 路由结构：
 * /            → 重定向到 /users
 * /login       → 登录页（仅未登录可访问）
 * /users       → 用户列表页（需要 UsersRead 权限）
 * /users/create → 创建用户页（需要 UsersWrite 权限）
 * /403         → 无权限提示页
 * 其他         → 兜底重定向到 /login
 *
 * 路由守卫（beforeEach）负责：
 * 1. 每次导航时校验 token 有效性
 * 2. 未登录用户访问需认证页面时重定向到 /login
 * 3. 已登录用户访问登录页时重定向到 /users
 * 4. 权限不足时重定向到 /403
 */
import { useAuthStore } from "@/stores/auth";
import { Permission, createAppRouter } from "@shared";

const router = createAppRouter({
  app: "admin",
  homePath: "/users", // 已登录用户访问登录页时跳转到用户管理页
  noPermission: "403", // admin 无权限跳 403 页（用户端默认回跳首个菜单）
  useAuthStore,
  routes: [
    {
      path: "/",
      redirect: "/users", // 根路径默认跳转到用户管理页
    },
    {
      path: "/login",
      name: "Login",
      // 路由懒加载：() => import(...) 只有访问该路由时才加载组件，减少首屏体积
      component: () => import("@/views/Login.vue"),
      meta: { requiresGuest: true }, // 标记：仅允许未登录用户访问
    },
    {
      path: "/users",
      name: "Users",
      component: () => import("@/admin/views/Users.vue"),
      meta: {
        requiresAuth: true,               // 标记：需要登录
        permissions: [Permission.UsersRead], // 标记：需要 UsersRead 权限
      },
    },
    {
      path: "/users/create",
      name: "CreateUser",
      component: () => import("@/admin/views/CreateUser.vue"),
      meta: {
        requiresAuth: true,
        permissions: [Permission.UsersWrite],
      },
    },
    {
      path: "/403",
      name: "NoPermission",
      component: () => import("@/admin/views/NoPermission.vue"),
      meta: {
        requiresAuth: true, // 需要登录才能看到此页面
      },
    },
    // 兜底：未匹配路径一律回登录页（守卫会决定已登录用户的去向），避免空白页
    {
      path: "/:pathMatch(.*)*", // :pathMatch(.*)* 匹配所有路径
      redirect: "/login",
    },
  ],
});

export default router;
