/**
 * 路由工厂：createAppRouter
 *
 * 7 个前端应用的路由守卫（initAuth → requiresAuth → requiresGuest → 权限检查）
 * 完全一致，收敛到本工厂；各应用 router/index.ts 只保留应用专属的 routes 数组。
 *
 * 差异点通过参数控制：
 * - `homePath`：已登录用户访问 /login 时跳转的首页（admin = /users，其余 = /<app>）
 * - `noPermission`："menu" = 回跳当前角色首个菜单（用户端默认），"403" = 跳 /403（admin）
 *
 * 注意：守卫使用各应用自己的 auth store（createAuthStore 工厂创建），
 * 因此调用方必须传入本应用的 `useAuthStore`。
 */
import { createRouter, createWebHistory, type RouteRecordRaw, type Router } from "vue-router";
import { getMenusByRole } from "./roles";
import type { Permission } from "./types";

/** 守卫所需的 auth store 最小接口（各应用 createAuthStore 产物均满足） */
export interface AuthStoreLike {
    isAuthenticated: boolean;
    userRole: string;
    hasPermission: (permission: Permission) => boolean;
    initAuth: () => Promise<void>;
}

export interface AppRouterOptions {
    /** 应用名（生产 base 前缀，如 "admin" → "/admin/"） */
    app: string;
    /** 应用专属路由表 */
    routes: RouteRecordRaw[];
    /**
     * 当前应用的 auth store 获取器（由应用传入）。
     * 类型取宽 `() => any`：createAuthStore 返回的 StoreDefinition 泛型被擦除，
     * 无法结构匹配 AuthStoreLike（与 registerAuthStoreGetter 同款处理）。
     */
    useAuthStore: () => AuthStoreLike | any;
    /** 已登录用户访问 /login 时跳转的首页（默认 `/${app}`） */
    homePath?: string;
    /** 无权限跳转策略：默认 "menu"（用户端，回跳首个菜单）；admin 用 "403" */
    noPermission?: "menu" | "403";
}

export function createAppRouter(options: AppRouterOptions): Router {
    const {app, routes, useAuthStore} = options;
    const homePath = options.homePath ?? `/${app}`;

    const router = createRouter({
        // 生产环境由后端托管在 /<app> 路径下，历史模式需带上该前缀
        history: createWebHistory(import.meta.env.PROD ? `/${app}/` : "/"),
        routes,
    });

    /**
     * 全局前置路由守卫（7 应用统一）：
     * 1. 初始化认证状态（含服务端校验）
     * 2. 未登录访问需认证页面 → /login
     * 3. 已登录访问登录页 → 首页
     * 4. 权限不足 → 回跳首个有权限菜单（admin 跳 /403）
     */
    router.beforeEach(async (to, _from, next) => {
        const authStore = useAuthStore();

        await authStore.initAuth();

        // 需要登录但用户未登录 → 跳转登录页
        if (to.meta.requiresAuth && !authStore.isAuthenticated) {
            next("/login");
            return;
        }

        // 已登录用户访问登录页 → 跳转首页
        if (to.meta.requiresGuest && authStore.isAuthenticated) {
            next(homePath);
            return;
        }

        // 权限检查（"任一权限满足即通过"）
        if (to.meta.permissions) {
            const requiredPermissions = to.meta.permissions as Permission[];
            const hasPermission = requiredPermissions.some((permission) =>
                authStore.hasPermission(permission)
            );

            if (!hasPermission) {
                if (options.noPermission === "403") {
                    // admin：跳无权限页（避免重定向到受限页面形成死循环）
                    next("/403");
                    return;
                }
                // 用户端：跳转当前角色在菜单中第一个有权限的面板（注册表驱动）
                const menus = getMenusByRole(authStore.userRole, "user");
                const fallback = menus[0]?.children?.[0]?.path ?? menus[0]?.path;
                next(fallback ?? "/login");
                return;
            }
        }

        next();
    });

    return router;
}
