/**
 * @module stores/auth
 * @description 前端应用（mario）认证 Store
 *
 * 使用 @shared 的公共认证实现，限定本应用只接受 `mario` 角色登录。
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-mario",
  appKind: "user",
  allowedRoles: ["mario"],
  authApi,
});

registerAuthStoreGetter(() => useAuthStore());