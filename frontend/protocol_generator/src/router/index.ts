/**
 * 通信协议生成应用路由配置
 *
 * 路由结构：
 * /                     → 重定向到 /login
 * /login                → 登录页
 * /protocol_generator/editor → 协议编辑（需要 ProtocolGeneratorMonitor 权限）
 * /protocol_generator/csv    → CSV 参数表编辑（需要 ProtocolGeneratorMonitor 权限）
 * 其他                  → 兜底重定向到 /login
 *
 * 生产环境路径前缀为 /protocol_generator/，开发环境为 /。
 */
import {useAuthStore} from "@/stores/auth";
import {Permission, createAppRouter} from "@shared";

const router = createAppRouter({
    app: "protocol_generator",
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
            path: "/protocol_generator/editor",
            name: "ProtocolGeneratorEditor",
            component: () => import("@/protocol_generator/views/Editor.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.ProtocolGeneratorMonitor],
            },
        },
        {
            path: "/protocol_generator/csv",
            name: "ProtocolGeneratorCsv",
            component: () => import("@/protocol_generator/views/Csv.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.ProtocolGeneratorMonitor],
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