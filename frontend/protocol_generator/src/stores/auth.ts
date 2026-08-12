/**
 * 通信协议生成应用（protocol_generator）认证 Store
 *
 * 使用 @shared 工厂函数创建，仅放行 protocol_generator 角色。
 * 通过 registerAuthStoreGetter 注册到 shared 注册表。
 */
import {createAuthStore, registerAuthStoreGetter} from "@shared";
import {authApi} from "@/api";

export const useAuthStore = createAuthStore({
    id: "auth-protocol-generator",
    appKind: "user",
    allowedRoles: ["protocol_generator"],
    authApi,
});

registerAuthStoreGetter(() => useAuthStore());