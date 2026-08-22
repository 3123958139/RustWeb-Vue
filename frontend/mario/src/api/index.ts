/**
 * @module api
 * @description API 客户端装配（mario 应用）
 *
 * - `api`：Axios 实例（注入 token 与 401 跳转拦截器，baseURL 默认 /api）
 * - `authApi`：登录/登出/用户信息
 * - `marioApi`：游戏成绩 API（高分榜 / 提交 / 统计）
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createMarioApi } from "@/mario/api/mario";

export const api = createApiClient(import.meta.env.PROD ? "/mario/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const marioApi = createMarioApi();