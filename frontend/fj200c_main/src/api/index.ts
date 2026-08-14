/**
 * @module api
 * @description API 客户端装配（fj200c_main 应用）
 *
 * 组装共享的认证 API 与角色 API facade：
 * - `api`：Axios 实例（baseURL 随生产/开发环境切换，注入 token 与 401 跳转拦截器）
 * - `authApi`：登录/登出/用户信息
 * - `fj200cMainApi`：本角色业务 API（服务启停/指令/配置/CSV/报表等）
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createFj200cMainApi } from "@/fj200c_main/api/fj200c_main";

export const api = createApiClient(import.meta.env.PROD ? "/fj200c_main/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const fj200cMainApi = createFj200cMainApi();
