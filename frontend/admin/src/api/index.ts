/**
 * 管理后台 API 客户端封装
 *
 * 复用 @shared 提供的公共工厂函数：
 * - createApiClient: 创建 Axios 实例，配置 base URL 与 401 拦截器
 * - createAuthApi: 创建认证相关 API（login/logout/verifyToken）
 *
 * 401 拦截器：后端返回 401 时自动跳转到本应用登录页
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createUsersApi } from "@/admin/api/users";
import { createSettingsApi } from "@/admin/api/settings";

/**
 * 创建 Axios 实例
 *
 * 开发环境代理到 /login（Vite proxy 将 /api 转发到 localhost:3000），
 * 生产环境 401 拦截跳转到 /admin/login（后端托管路径）。
 */
export const api = createApiClient(import.meta.env.PROD ? "/admin/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

/** 认证 API（登录、登出、token 校验） */
export const authApi = createAuthApi();

/** 用户管理 API（CRUD 操作，仅 admin 角色可用） */
export const usersApi = createUsersApi();

/** 系统设置 API（初始密码查询停用开关等，仅 admin 角色可用） */
export const settingsApi = createSettingsApi();
