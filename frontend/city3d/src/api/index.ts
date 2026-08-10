import { createApiClient, createAuthApi, setApiInstance } from "@shared";
import { createCity3dApi } from "@/city3d/api/city3d";

export const api = createApiClient(import.meta.env.PROD ? "/city3d/login" : "/login");

/** 注入 orval 生成的客户端使用的 Axios 实例（token 注入 + 401 跳转） */
setApiInstance(api);

export const authApi = createAuthApi();

export const city3dApi = createCity3dApi();
