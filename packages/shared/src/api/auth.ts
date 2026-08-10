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
  };
}

export type AuthApi = ReturnType<typeof createAuthApi>;
