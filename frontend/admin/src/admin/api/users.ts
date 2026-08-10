/**
 * 管理后台用户管理 API
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 请求自动走 @shared 注入的 Axios 实例（token 注入 + 401 跳转）。
 * 后端由 role_middleware 保护（仅 admin 角色可调用）。
 */
import { getAdmin } from "@shared/api/generated";
import type { CreateUserRequest, UpdateUserRoleRequest } from "@shared";

/** 创建用户管理 API 对象（无需传参） */
export function createUsersApi() {
  const api = getAdmin();
  return {
    /** 获取所有用户列表 */
    async getUsers() {
      return api.adminListUsers();
    },

    /** 创建新用户 */
    async createUser(data: CreateUserRequest) {
      return api.adminCreateUser(data);
    },

    /** 更新用户角色 */
    async updateUserRole(id: string, role: string) {
      return api.adminUpdateUserRole(id, { role } satisfies UpdateUserRoleRequest);
    },

    /** 删除用户 */
    async deleteUser(id: string) {
      return api.adminDeleteUser(id);
    },
  };
}

/** 用户管理 API 类型 */
export type UsersApi = ReturnType<typeof createUsersApi>;
