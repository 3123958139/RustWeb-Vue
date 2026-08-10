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