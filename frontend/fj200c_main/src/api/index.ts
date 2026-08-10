import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createFj200cMainApi } from "@/fj200c_main/api/fj200c_main";

export const api = createApiClient(import.meta.env.PROD ? "/fj200c_main/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const fj200cMainApi = createFj200cMainApi();
