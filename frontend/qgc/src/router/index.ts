/**
 * 飞控地面站应用路由配置
 *
 * 路由结构：
 * /           → 重定向到 /login
 * /login      → 登录页
 * /qgc        → 飞控地面站（仪表盘/地图与任务/配置/帮助，需要 QgcMonitor 权限）
 * 其他        → 兜底重定向到 /login
 *
 * 生产环境路径前缀为 /qgc/，开发环境为 /。
 */
import {useAuthStore} from "@/stores/auth";
import {createAppRouter, Permission} from "@shared";

const router = createAppRouter({
    app: "qgc",
    useAuthStore,
    routes: [
        {
            path: "/",
            redirect: "/login",
        },
        {
            path: "/qgc",
            name: "QgcHome",
            redirect: "/qgc/monitor",
        },
        {
            path: "/login",
            name: "Login",
            component: () => import("@/views/Login.vue"),
            meta: {requiresGuest: true},
        },
        {
            path: "/qgc/monitor",
            name: "QgcMonitor",
            component: () => import("@/qgc/views/Monitor.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.QgcMonitor],
            },
        },
        {
            path: "/qgc/map",
            name: "QgcMap",
            component: () => import("@/qgc/views/Map.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.QgcMonitor],
            },
        },
        {
            path: "/qgc/config",
            name: "QgcConfig",
            component: () => import("@/qgc/views/Config.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.QgcMonitor],
            },
        },
        {
            path: "/qgc/help",
            name: "QgcHelp",
            component: () => import("@/qgc/views/Help.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.QgcMonitor],
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
