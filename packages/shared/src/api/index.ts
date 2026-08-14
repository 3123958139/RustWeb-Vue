/**
 * @module api/index
 * @description API 客户端工厂模块
 *
 * 本模块提供创建 Axios 实例的工厂函数，用于所有前端应用的 API 请求。
 *
 * ## 功能特性
 *
 * ### 请求拦截器
 * - 自动添加 JWT Token 到请求头
 * - 从 localStorage 获取最新 token
 *
 * ### 响应拦截器
 * - 401 错误自动清除会话并跳转登录页
 * - 其他错误正常抛出
 *
 * ### 配置
 * - baseURL: 从环境变量读取或默认 `/api`
 * - timeout: 10 秒
 *
 * ## 使用场景
 * 每个前端应用在初始化时创建自己的 API 客户端：
 * ```typescript
 * // 在 frontend/fj200c_information/api/index.ts 中
 * import { createApiClient } from '@shared/api';
 * export const apiClient = createApiClient('/login');
 * ```
 *
 * ## 与角色应用的关系
 * - shared 包提供工厂函数
 * - 角色应用创建具体实例
 * - 角色专属 API 使用实例发起请求
 */

import axios, { AxiosInstance } from "axios";
import { clearSession, getSessionToken } from "../session";

/**
 * @function createApiClient
 * @description 创建 API 客户端实例
 *
 * @param {string} loginPath - 401 错误时的跳转路径
 *   - 用户端应用："/login"
 *   - 管理端应用："/admin/login"
 * @returns {AxiosInstance} Axios 实例
 *
 * 配置说明：
 * - baseURL: API 基础路径，优先使用环境变量 `VITE_API_BASE_URL`，默认 "/api"
 * - timeout: 请求超时时间，10 秒（10000 毫秒）
 *
 * 拦截器说明：
 * 1. 请求拦截器：在每个请求发送前，自动添加 Authorization 头
 * 2. 响应拦截器：处理 401 错误，清除会话并跳转登录页
 *
 * 使用示例：
 * ```typescript
 * // 创建实例
 * const api = createApiClient('/login');
 *
 * // 发起请求
 * const response = await api.get('/users');
 *
 * // POST 请求
 * const response = await api.post('/auth/login', { email, password });
 * ```
 */
export function createApiClient(loginPath: string): AxiosInstance {
  // 创建 Axios 实例
  const api = axios.create({
    // API 基础路径
    // import.meta.env 是 Vite 的环境变量对象
    // VITE_API_BASE_URL 是在 .env 文件中定义的环境变量
    baseURL: import.meta.env.VITE_API_BASE_URL || "/api",
    // 请求超时时间（毫秒）
    timeout: 10000,
  });

  // ============ 请求拦截器 ============
  // 在每个请求发送前执行
  // 用途：自动添加 JWT Token 到请求头
  // synchronous: true —— 拦截器在调用栈内同步执行，确保 token 在发起调用的同一同步块内被读取
  // （登出流程会先清会话，若拦截器在微任务中异步读取 localStorage，登出请求将丢失 token 而 401，
  //   导致后端收不到停止服务的指令，后台线程继续运行）
  api.interceptors.request.use(
    // 成功回调：处理请求配置
    (config) => {
      // 从 localStorage 获取 JWT Token
      const token = getSessionToken();
      if (token) {
        // 添加 Authorization 头
        // 格式：Bearer {token}
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    },
    // 错误回调：处理请求配置错误
    (error) => Promise.reject(error),
    {synchronous: true}
  );

  // ============ 响应拦截器 ============
  // 在每个响应接收后执行
  // 用途：统一处理错误，特别是 401 未授权错误
  api.interceptors.response.use(
    // 成功回调：直接返回响应
    (response) => response,
    // 错误回调：处理响应错误
    (error) => {
      // 检查是否为 401 未授权错误
      if (error.response?.status === 401) {
        // 清除本地会话
        clearSession();
        // 跳转到登录页
        // 使用 window.location.href 而不是 Vue Router，
        // 因为此时可能不在 Vue 应用上下文中
        window.location.href = loginPath;
      }
      // 抛出错误，让调用方处理
      return Promise.reject(error);
    }
  );

  return api;
}
