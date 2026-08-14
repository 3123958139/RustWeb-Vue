/**
 * 认证 API 工厂。
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 请求自动走 customInstance mutator（JWT 注入 + 401 跳转）。
 */
import { getAuth } from "./generated";
import type { LoginRequest } from "./generated/model";

export function createAuthApi() {
  return {
    /** 用户登录 */
    async login(data: LoginRequest) {
      return getAuth().authLogin(data);
    },

    /** 获取当前用户信息 */
    async getProfile() {
      return getAuth().authGetProfile();
    },

    /**
     * 退出登录 / 角色清理（公共组件，所有角色通用）：
     * 后端按角色隔离停止后台线程与资源，有且只有 keepRole 角色保持运行。
     * keepRole 缺省时停止所有角色的服务（退出登录场景）。
     * token 显式传入：登出会先清会话，显式携带保证请求不丢失凭证（双保险）。
     */
    async logout(keepRole?: string, token?: string) {
      const options = token
        ? {
            headers: {
              Authorization: `Bearer ${token}`,
              "Content-Type": "application/json",
            },
          }
        : undefined;
      return getAuth().authLogout({keep_role: keepRole ?? null}, options);
    },
  };
}

export type AuthApi = ReturnType<typeof createAuthApi>;
