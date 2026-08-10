/**
 * 管理后台系统设置 API
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 请求自动走 @shared 注入的 Axios 实例（token 注入 + 401 跳转）。
 * 后端由 role_middleware 保护（仅 admin 角色可调用）。
 */
import { getAdmin } from "@shared/api/generated";
import type { PwdRouteStatus } from "@shared/api/generated";

/** 创建系统设置 API 对象（无需传参） */
export function createSettingsApi() {
  const api = getAdmin();
  return {
    /** 获取初始密码查询路由（GET /admin/pwd）停用状态 */
    async getPwdRouteStatus() {
      return api.adminGetPwdRouteStatus();
    },

    /** 设置初始密码查询路由（GET /admin/pwd）停用状态 */
    async setPwdRouteStatus(disabled: boolean) {
      return api.adminSetPwdRouteStatus({ disabled } satisfies PwdRouteStatus);
    },
  };
}

/** 系统设置 API 类型 */
export type SettingsApi = ReturnType<typeof createSettingsApi>;
