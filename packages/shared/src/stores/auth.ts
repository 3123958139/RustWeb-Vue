import {defineStore, type StoreDefinition} from "pinia";
import {ref, computed} from "vue";
import type {MenuItem, Permission, User} from "../types";
import {clearSession, getSessionToken, loadSession, saveSession, SESSION_KEY, LEGACY_KEYS} from "../session";
import {getMenusByRole, getPermissionsByRole, loadRoleRegistry} from "../roles";
import type {AuthApi} from "../api/auth";

// ============ 应用 auth store 注册表 ============
// 各应用在 stores/auth.ts 中通过 registerAuthStoreGetter 登记自己的 store 获取器；
// 公共组件（如 AppNavbar）通过 getAppAuthStore() 拿到当前应用的 store 实例（无需传参）。
let appAuthStoreGetter: (() => any) | null = null;

export function registerAuthStoreGetter(getter: () => any) {
    appAuthStoreGetter = getter;
}

/** 当前应用的 auth store 实例（未登记 → undefined） */
export function getAppAuthStore<T>(): T | undefined {
    return appAuthStoreGetter?.();
}

export interface AuthStoreOptions {
    /** store id（各应用必须不同，如 "auth-fj200c_information"、"auth-fw150"、"auth-admin"） */
    id: string;
    /** 应用侧分类："user"=用户端应用（fj200c_information / fw150 等角色面板），"admin"=管理端应用（决定菜单来源） */
    appKind: "user" | "admin";
    /** 本应用允许登录的角色（用户端=[角色 key...]，管理端=[admin]） */
    allowedRoles: string[];
    /** 认证 API（由应用侧注入，避免 shared 直接依赖 axios 实例） */
    authApi: AuthApi;
}

/**
 * 认证 store 工厂（公用代码）：会话管理 + 服务端校验 + 角色权限/菜单（注册表驱动）。
 * fj200c_information 与 admin 通过参数差异化，业务逻辑只维护一份。
 */
export function createAuthStore(options: AuthStoreOptions): StoreDefinition {
    const {id, appKind, allowedRoles, authApi} = options;

    return defineStore(id, () => {
        const user = ref<User | null>(null);
        const token = ref<string | null>(null);
        const permissions = ref<Permission[]>([]);
        const menuItems = ref<MenuItem[]>([]);

        // 仅放行本应用允许的角色（管理员会话属于管理后台，用户端视为未登录，反之亦然）
        const isAuthenticated = computed(
            () => !!token.value && !!user.value && allowedRoles.includes(user.value.role)
        );

        const userRole = computed(() => user.value?.role ?? "");

        const hasPermission = (permission: Permission): boolean => permissions.value.includes(permission);

        const hasAnyPermission = (requiredPermissions: Permission[]): boolean =>
            requiredPermissions.some((permission) => hasPermission(permission));

        const hasAllPermissions = (requiredPermissions: Permission[]): boolean =>
            requiredPermissions.every((permission) => hasPermission(permission));

        // 根据当前 token/user 刷新权限与菜单（角色注册表驱动）
        const refreshAuthState = () => {
            if (!user.value) {
                permissions.value = [];
                menuItems.value = [];
                return;
            }
            permissions.value = getPermissionsByRole(user.value.role);
            menuItems.value = getMenusByRole(user.value.role, appKind);
        };

        let initPromise: Promise<void> | null = null;
        let initialized = false;

        // 从本地会话恢复 + 服务端校验（幂等：首次调用后不再重复请求）
        const initAuth = async (): Promise<void> => {
            if (initialized) return;
            if (initPromise) return initPromise;
            initPromise = (async () => {
                // 角色注册表（key/name/permissions）来自后端 /api/meta/roles（orval 单一来源），
                // 先加载再计算权限/菜单；失败时缓存为空数组，不影响登录流程
                await loadRoleRegistry();

                const session = loadSession();
                if (!session) {
                    clearSession();
                    user.value = null;
                    token.value = null;
                    refreshAuthState();
                    return;
                }

                token.value = session.token;
                user.value = session.user;
                refreshAuthState();

                // 服务端校验：token 无效/用户被删除则登出；成功则用服务端数据刷新本地，
                // 保证展示的用户信息永远与服务端一致（角色变更、封号等立即生效）
                try {
                    const response = await authApi.getProfile();
                    if (response.success && response.data) {
                        if (!allowedRoles.includes(response.data.role)) {
                            // 共享会话的用户不属于本应用：不破坏另一端的会话，仅本应用视为未登录
                            user.value = null;
                            token.value = null;
                            refreshAuthState();
                            return;
                        }
                        user.value = response.data;
                        if (token.value) saveSession(token.value, response.data);
                        refreshAuthState();
                    } else {
                        clearSession();
                        user.value = null;
                        token.value = null;
                        refreshAuthState();
                    }
                } catch {
                    // 401 由 API 拦截器统一清理并跳转；其他异常保留本地会话，避免离线误登出
                    user.value = null;
                    token.value = null;
                    refreshAuthState();
                }
            })();
            try {
                await initPromise;
            } finally {
                initPromise = null;
                initialized = true;
            }
        };

        // 其他标签页切换用户时实时同步（storage 事件只在别的标签页触发）
        const syncFromStorage = () => {
            const prevUserId = user.value?.id ?? null;
            const session = loadSession();
            if (session && allowedRoles.includes(session.user.role)) {
                token.value = session.token;
                user.value = session.user;
                refreshAuthState();
            } else {
                // 其他角色会话（如管理员）：不破坏共享会话，仅本应用视为未登录
                user.value = null;
                token.value = null;
                refreshAuthState();
            }
            // 其他标签页切换了账号/角色（共享会话被新会话覆盖）：
            // 通知后端停止其他角色的线程与资源，保留新会话当前角色的线程
            if (session && prevUserId !== session.user.id) {
                stopServices(session.user.role);
            }
        };

        const handleStorage = (event: StorageEvent) => {
            if (
                event.key === null ||
                event.key === SESSION_KEY ||
                (event.key && LEGACY_KEYS.includes(event.key as (typeof LEGACY_KEYS)[number]))
            ) {
                initialized = false;
                syncFromStorage();
            }
        };

        window.addEventListener("storage", handleStorage);

        // 登录
        const login = async (email: string, password: string) => {
            try {
                // 登录前确保注册表已加载（幂等，仅首次发请求）
                await loadRoleRegistry();
                const response = await authApi.login({email, password});
                if (response.success && response.data) {
                    // 切换账号/角色：先停掉旧会话的线程与资源，保留新账号角色的线程
                    // （此时请求仍携带旧 token，幂等）
                    if (!user.value || user.value.id !== response.data.user.id) {
                        stopServices(response.data.user.role);
                    }
                    token.value = response.data.token;
                    user.value = response.data.user;
                    saveSession(response.data.token, response.data.user);
                    refreshAuthState();
                    return {success: true as const};
                }
                return {success: false as const, message: response.message};
            } catch (error: any) {
                return {
                    success: false as const,
                    message: error.response?.data?.message || "登录失败",
                };
            }
        };

        // 通知后端按角色停止其他角色的后台线程与资源（公共组件，所有角色通用）
        // 有且只有 keepRole 角色保持运行；keepRole 缺省 = 停止所有角色（退出登录场景）。
        // 调用时机：退出登录 / 切换账号 / 切换角色 / 跨标签页会话变更 / 启动服务排他。
        // 后端 POST /api/auth/logout 统一按角色停止 fj200c_information / fj200c_main / ftj1c 的服务线程。
        // token 显式传入：登出/切换时会话即将被清除或覆盖，请求拦截器可能读不到旧 token。
        const stopServices = (keepRole?: string) => {
            const token = getSessionToken();
            // 无会话 token 时请求必然 401 并触发登录页跳转，直接跳过
            if (!token) return;
            authApi.logout(keepRole, token).catch(() => {});
        };

        // 登出（统一会话键，所有应用同步退出）
        const logout = () => {
            // 先通知后端停止所有角色的后台线程与资源（登出后无当前角色，全部退出）。
            // 必须在清会话前捕获 token 并显式传给请求：axios 请求拦截器虽有同步配置兜底，
            // 显式携带 token 保证即便时序变化也不丢失凭证。
            const token = getSessionToken();
            if (token) {
                authApi.logout(undefined, token).catch(() => {});
            }
            user.value = null;
            token.value = null;
            permissions.value = [];
            menuItems.value = [];
            clearSession();
        };

        // 刷新用户信息（每次调用都重新拉取服务端数据）
        const fetchProfile = async () => {
            try {
                const response = await authApi.getProfile();
                if (response.success && response.data) {
                    if (!allowedRoles.includes(response.data.role)) {
                        user.value = null;
                        token.value = null;
                        refreshAuthState();
                        return;
                    }
                    user.value = response.data;
                    if (token.value) saveSession(token.value, response.data);
                    refreshAuthState();
                }
            } catch {
                // 401 由 API 拦截器统一处理；其他异常保留本地会话
            }
        };

        return {
            user,
            token,
            permissions,
            menuItems,
            isAuthenticated,
            userRole,
            hasPermission,
            hasAnyPermission,
            hasAllPermissions,
            initAuth,
            login,
            logout,
            stopServices,
            fetchProfile,
        };
    });
}
