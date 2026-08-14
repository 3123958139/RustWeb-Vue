/**
 * @module api
 * @description API 客户端装配（fj200c_information 应用）
 *
 * 组装共享的认证 API 与角色 API facade：
 * - `api`：Axios 实例（baseURL 随生产/开发环境切换，注入 token 与 401 跳转拦截器）
 * - `authApi`：登录/登出/用户信息
 * - `fj200c_informationApi`：本角色业务 API（服务启停/命令/配置/CSV 等）
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createFj200cInformationApi } from "@/fj200c_information/api/fj200c_information";

export const api = createApiClient(import.meta.env.PROD ? "/fj200c_information/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const fj200c_informationApi = createFj200cInformationApi();
