/**
 * 管理后台认证 Store（Pinia）
 *
 * 使用 @shared 提供的工厂函数 createAuthStore 创建，仅放行 admin 角色。
 * 通过 registerAuthStoreGetter 注册到 shared 注册表，
 * 使公共组件（如 AppNavbar）能通过 getAppAuthStore() 获取本应用的 store 实例。
 *
 * Store 提供的状态和方法：
 * - isAuthenticated: 是否已认证
 * - user: 当前登录用户信息
 * - userRole: 当前用户角色
 * - hasPermission(perm): 检查是否有某权限
 * - login(email, password): 登录
 * - logout(): 登出
 * - initAuth(): 初始化认证状态
 */
import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

/**
 * 创建认证 Store
 *
 * @param id        - Pinia Store 唯一标识符（不同应用不能重复）
 * @param appKind   - 应用类型，"admin" 表示管理后台
 * @param allowedRoles - 允许登录的角色列表（管理后台仅允许 admin）
 * @param authApi   - 认证 API 实例，用于登录/校验请求
 */
export const useAuthStore = createAuthStore({
  id: "auth-admin",
  appKind: "admin",
  allowedRoles: ["admin"],
  authApi,
});

// 登记到 shared 注册表：公共组件（AppNavbar 等）通过 getAppAuthStore() 获取本应用 store
registerAuthStoreGetter(() => useAuthStore());
