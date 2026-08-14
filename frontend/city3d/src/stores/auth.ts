/**
 * @module stores/auth
 * @description 前端应用（city3d）认证 Store
 *
 * 使用 @shared 的公共认证实现，限定本应用只接受 `city3d` 角色登录。
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-city3d",
  appKind: "user",
  allowedRoles: ["city3d"],
  authApi,
});

registerAuthStoreGetter(() => useAuthStore());