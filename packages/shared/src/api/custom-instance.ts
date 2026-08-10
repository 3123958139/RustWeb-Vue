/**
 * orval 生成的 API 客户端使用的自定义 axios 实例（mutator）。
 *
 * 复用 createApiClient 的拦截器能力（JWT 注入 + 401 跳转），
 * 由各前端应用通过 setApiInstance 注入自己的实例（登录跳转路径因应用而异）。
 */
import type { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";
import { createApiClient } from "./index";

let apiInstance: AxiosInstance | null = null;

/** 由各前端应用注入自己的 API 实例 */
export function setApiInstance(instance: AxiosInstance) {
  apiInstance = instance;
}

/** 获取当前 API 实例（未注入时创建默认实例，跳转 /login） */
export function getApiInstance(): AxiosInstance {
  if (!apiInstance) {
    apiInstance = createApiClient("/login");
  }
  return apiInstance;
}

/**
 * orval mutator：所有生成的请求都走这里。
 * 返回 data 本身（即 ApiResponse 包装），与现有 facade 的 `res.data` 语义一致。
 */
export const customInstance = <T>(
  config: AxiosRequestConfig,
  options?: AxiosRequestConfig
): Promise<T> => {
  const instance = getApiInstance();
  const baseURL = instance.defaults.baseURL || "";
  let url = config.url ?? "";

  // OpenAPI spec 中的 url 是完整路径（如 /api/auth/login），
  // 而实例已配置 baseURL=/api，剥离前缀避免重复。
  if (baseURL && url.startsWith(baseURL)) {
    url = url.slice(baseURL.length);
  }

  return instance({ ...config, url, ...options }).then(({ data }) => data);
};

export type ErrorType<Error> = AxiosError<Error>;
export type BodyType<BodyData> = BodyData;
