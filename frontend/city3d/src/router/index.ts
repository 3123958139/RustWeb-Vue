/**
 * @module router
 * @description 前端应用（city3d）路由配置
 *
 * 路由结构：
 * - `/login`：登录页（未登录访问）
 * - `/city3d/main`：3D 城市场景（需登录 + City3dView 权限）
 * - `/city3d/data`：数据面板（需登录 + City3dView 权限）
 * - 其余路径重定向到登录页
 */
import {useAuthStore} from "@/stores/auth";
import {Permission, createAppRouter} from "@shared";

const router = createAppRouter({
    app: "city3d",
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
            path: "/city3d",
            redirect: "/city3d/main",
        },
        {
            path: "/city3d/main",
            name: "CityScene",
            component: () => import("@/city3d/views/CityScene.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.City3dView],
            },
        },
        {
            path: "/city3d/data",
            name: "DataPanel",
            component: () => import("@/city3d/views/DataPanel.vue"),
            meta: {
                requiresAuth: true,
                permissions: [Permission.City3dView],
            },
        },
        {
            path: "/:pathMatch(.*)*",
            redirect: "/login",
        },
    ],
});

export default router;