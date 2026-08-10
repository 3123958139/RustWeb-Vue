/**
 * 设备台账应用路由配置
 *
 * 路由结构：
 * /           → 重定向到 /login
 * /login      → 登录页
 * /fw150      → 设备台账面板（需要 Fw150Read 权限）
 * 其他        → 兜底重定向到 /login
 *
 * 生产环境路径前缀为 /fw150/，开发环境为 /。
 */
import {useAuthStore} from "@/stores/auth";
import {Permission, createAppRouter} from "@shared";

const router = createAppRouter({
    app: "fw150",
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
            meta: {requiresGuest: true},
        },
        {
            path: "/fw150",
            name: "Fw150Panel",
            component: () => import("@/fw150/views/Panel.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.Fw150Monitor],
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
