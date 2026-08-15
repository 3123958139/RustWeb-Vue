/**
 * 飞控地面站应用（qgc）认证 Store
 *
 * 使用 @shared 工厂函数创建，仅放行 qgc 角色。
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-qgc",
  appKind: "user",
  allowedRoles: ["qgc"],
  authApi,
});

registerAuthStoreGetter(() => useAuthStore());
