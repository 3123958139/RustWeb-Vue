import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createFj200cInformationApi } from "@/fj200c_information/api/fj200c_information";

export const api = createApiClient(import.meta.env.PROD ? "/fj200c_information/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const fj200c_informationApi = createFj200cInformationApi();
