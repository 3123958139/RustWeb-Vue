/**
 * @module @rustweb/shared
 * @description 共享包入口文件 - 为所有 Vue 前端应用提供公共代码
 *
 * ## 架构说明
 * 本包是 RustWeb-Vue 项目的共享代码包，被 5 个 Vue 前端应用引用：
 * - frontend/fj200c_information（发动机监控）
 * - frontend/admin（管理后台）
 * - frontend/fw150（设备台账）
 * - frontend/ftj1c（UDP通信监控）
 * - frontend/city3d（城市 3D 展示）
 *
 * ## 设计原则
 * 1. **跨角色公共代码**：只包含所有角色共享的逻辑（会话管理、类型定义、角色注册表、工厂函数）
 * 2. **角色专属代码隔离**：每个角色的 API、组件、路由位于各自的前端应用中
 * 3. **注册表驱动**：通过角色注册表动态生成菜单和权限，无需硬编码
 * 4. **统一会话管理**：所有应用共享同一份 localStorage 会话，确保登录态一致性
 *
 * ## 模块结构
 * - `types.ts` - TypeScript 类型定义（用户、权限、菜单、API请求/响应）
 * - `roles.ts` - 角色注册表（定义角色、权限、菜单）
 * - `session.ts` - 会话管理（localStorage 操作、旧版迁移）
 * - `stores/auth.ts` - Pinia Store 工厂（创建各应用的认证 Store）
 * - `api/index.ts` - Axios 客户端工厂（请求拦截、401处理）
 * - `api/auth.ts` - 认证 API 接口
 * - `template/` - 可复用的 Vue 组件（导航栏、模板面板）
 */

// ============ 模块导出 ============
// 统一导出所有公共模块，供各前端应用通过 `@shared` 别名引用
// 使用 `export *` 导出所有类型和函数，使用 `export { }` 导出特定值

// 类型定义模块 - 导出所有 TypeScript 接口和枚举
export * from "./types";

// 角色注册表模块 - 导出角色定义、菜单、权限相关函数
export * from "./roles";

// 会话管理模块 - 导出 localStorage 操作函数
export * from "./session";

// 路由工厂 - 统一 7 个应用的路由守卫逻辑
export {createAppRouter} from "./router";
export type {AppRouterOptions, AuthStoreLike} from "./router";

// 响应式工具 - 统一断点检测与布局配置
export {breakpoints, useResponsive, useLayoutConfig} from "./responsive";

// API 客户端工厂 - 导出 Axios 实例创建函数
// 注意：只导出工厂函数，不导出 axios 实例本身
export {createApiClient} from "./api";

// 认证 API 工厂 - 导出登录、获取用户信息等接口
export {createAuthApi} from "./api/auth";
// 导出 AuthApi 类型，供 TypeScript 类型推断使用
export type {AuthApi} from "./api/auth";

// orval 生成的 API 客户端（@shared/api/generated）使用的自定义实例：
// - setApiInstance: 各前端应用注入自己的 Axios 实例（登录跳转路径因应用而异）
// - getApiInstance: 获取当前实例
// - customInstance: orval mutator，所有生成的请求都走它
export {setApiInstance, getApiInstance, customInstance} from "./api/custom-instance";
// 生成代码的类型（如需在应用内直接使用类型，可 import from "@shared/api/generated"）

// 认证 Store 工厂 - 导出创建 Pinia Store 的工厂函数
// 以及用于跨应用获取 store 实例的工具函数
export {createAuthStore, getAppAuthStore, registerAuthStoreGetter} from "./stores/auth";
// 导出 AuthStoreOptions 类型，供各应用配置 store 时使用
export type {AuthStoreOptions} from "./stores/auth";

// 可复用 Vue 组件
// RoleTemplatePanel - 角色模板面板，用于演示如何创建新角色
export {default as RoleTemplatePanel} from "./template/TemplatePanel.vue";
// AppNavbar - 通用导航栏组件，根据角色注册表动态生成菜单
export {default as AppNavbar} from "./template/AppNavbar.vue";
// LoginPage - 通用登录页组件，各应用通过 props 复用统一模板
export {default as LoginPage} from "./template/LoginPage.vue";
