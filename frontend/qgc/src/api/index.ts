/**
 * 飞控地面站应用（qgc）API 客户端封装
 *
 * 复用 @shared 的公共工厂函数创建 Axios 实例和认证 API。
 * 同时导出 qgc 专属的业务 API。
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createQgcApi } from "@/qgc/api/qgc";

/** Axios 实例 */
export const api = createApiClient(import.meta.env.PROD ? "/qgc/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

/** 认证 API */
export const authApi = createAuthApi();

/** 飞控地面站业务 API */
export const qgcApi = createQgcApi();
