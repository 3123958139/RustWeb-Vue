/**
 * 通信协议生成应用（protocol_generator）API 客户端封装
 *
 * 复用 @shared 的公共工厂函数创建 Axios 实例和认证 API。
 * 同时导出 protocol_generator 专属的业务 API。
 */
import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createProtocolGeneratorApi } from "@/protocol_generator/api/protocol-generator";

/** Axios 实例 */
export const api = createApiClient(import.meta.env.PROD ? "/protocol_generator/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

/** 认证 API */
export const authApi = createAuthApi();

/** 通信协议生成业务 API */
export const protocolGeneratorApi = createProtocolGeneratorApi();