/**
 * UDP 通信监控应用（ftj1c）认证 Store
 *
 * 使用 @shared 工厂函数创建，仅放行 ftj1c 角色。
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-ftj1c",
  appKind: "user",
  allowedRoles: ["ftj1c"],
  authApi,
});

registerAuthStoreGetter(() => useAuthStore());
