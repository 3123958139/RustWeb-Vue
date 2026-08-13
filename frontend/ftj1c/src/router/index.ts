/**
 * UDP 通信监控应用路由配置
 *
 * 路由结构：
 * /           → 重定向到 /login
 * /login      → 登录页
 * /ftj1c      → UDP 通信监控面板（需要 Ftj1cMonitor 权限）
 * 其他        → 兜底重定向到 /login
 *
 * 生产环境路径前缀为 /ftj1c/，开发环境为 /。
 */
import {useAuthStore} from "@/stores/auth";
import {createAppRouter, Permission} from "@shared";

const router = createAppRouter({
    app: "ftj1c",
    useAuthStore,
    routes: [
        {
            path: "/",
            redirect: "/login",
        },
        {
            path: "/ftj1c",
            name: "Ftj1cHome",
            redirect: "/ftj1c/monitor",
        },
        {
            path: "/login",
            name: "Login",
            component: () => import("@/views/Login.vue"),
            meta: {requiresGuest: true},
        },
        {
            path: "/ftj1c/help",
            name: "Ftj1cHelp",
            component: () => import("@/ftj1c/views/Help.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.Ftj1cHelp],
            }
        },
        {
            path: "/ftj1c/monitor",
            name: "Ftj1cMonitor",
            component: () => import("@/ftj1c/views/Monitor.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.Ftj1cMonitor],
            },
        },
        // 兜底：未匹配路径重定向到登录页
        {
            path: "/:pathMatch(.*)*",
            redirect: "/login",
        },
    ],
});

export default router;
