/**
 * 发动机测控应用（fj200c_main）认证 Store
 *
 * 使用 @shared 工厂函数创建，仅放行 fj200c_main 角色。
 * 通过 registerAuthStoreGetter 注册到 shared 注册表，
 * 使公共组件（AppNavbar 等）能获取本应用的 store 实例。
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-fj200c_main",        // Pinia Store 唯一标识符
  appKind: "user",               // 应用类型（user 表示用户端）
  allowedRoles: ["fj200c_main"], // 仅允许 fj200c_main 角色登录
  authApi,                       // 认证 API 实例
});

// 登记到 shared 注册表：公共组件通过 getAppAuthStore() 获取本应用 store
registerAuthStoreGetter(() => useAuthStore());
