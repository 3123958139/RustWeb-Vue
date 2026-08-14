/**
 * @module api
 * @description API 客户端装配（city3d 应用）
 *
 * 组装共享的认证 API 与角色 API facade：
 * - `api`：Axios 实例（baseURL 随生产/开发环境切换，注入 token 与 401 跳转拦截器）
 * - `authApi`：登录/登出/用户信息
 * - `city3dApi`：本角色业务 API（建筑/区域/事件/概览）
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createCity3dApi } from "@/city3d/api/city3d";

export const api = createApiClient(import.meta.env.PROD ? "/city3d/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const city3dApi = createCity3dApi();
