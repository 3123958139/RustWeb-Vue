/**
 * 设备台账应用（fw150）认证 Store
 *
 * 使用 @shared 工厂函数创建，仅放行 fw150 角色。
 * 通过 registerAuthStoreGetter 注册到 shared 注册表。
 */
import {createAuthStore, registerAuthStoreGetter} from "@shared";
import {authApi} from "@/api";

export const useAuthStore = createAuthStore({
    id: "auth-fw150",
    appKind: "user",
    allowedRoles: ["fw150"],
    authApi,
});

registerAuthStoreGetter(() => useAuthStore());
