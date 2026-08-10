/**
 * @module types
 * @description 共享类型定义模块
 *
 * ## 类型来源
 *
 * - **DTO 类型（User / LoginRequest / CreateUserRequest 等）**：全部
 *   re-export 自 `@shared/api/generated/model`（orval 根据后端 OpenAPI
 *   规范生成）。后端 DTO 变更后运行 `npm run gen:api` 自动同步，
 *   避免手写副本与生成代码双份漂移。
 * - **注册表类型（Permission / RoleInfo）**：由后端
 *   `src/common/models.rs` / `src/roles.rs` 的 `ToSchema` 派生，经
 *   OpenAPI + orval 生成后 re-export，单一来源。
 * - **MenuItem**：纯前端导航概念，无 OpenAPI 描述，手写维护。
 *
 * 不要在下方手写任何与后端 DTO 重复的结构体——请优先从 generated 导入。
 */

// 本地引用 generated 类型（MenuItem 需要权限类型；re-export 不提供文件内绑定）
import type { Permission } from "./api/generated/model";

// ============ 注册表类型 ============

/**
 * @interface MenuItem
 * @description 菜单项接口
 *
 * 定义导航菜单的数据结构，支持多级嵌套。
 * 菜单由 `roles.ts` 的 MENU_CONFIG 驱动，根据用户权限动态生成。
 *
 * 菜单渲染逻辑：
 * 1. 根据用户角色获取对应的菜单列表
 * 2. 根据用户权限过滤无权限的菜单项
 * 3. 支持多级嵌套（最多 2 级：父菜单 + 子菜单）
 *
 * 注意：菜单结构是纯前端导航概念，后端 OpenAPI 无对应 schema，
 * 只能手写维护（与 `roles.ts` 的 MENU_CONFIG 同步）。
 */
export interface MenuItem {
    /** 菜单唯一标识符（用于 Vue Router 的 key） */
    id: string;
    /** 菜单显示标题 */
    title: string;
    /** 路由路径（Vue Router 格式） */
    path: string;
    /** Element Plus 图标名称（如 "User", "Setting"） */
    icon: string;
    /** 访问该菜单所需的权限点列表（满足任一即可） */
    permissions: Permission[];
    /** 子菜单列表（可选，支持多级嵌套） */
    children?: MenuItem[];
}

// ============ DTO 类型（re-export 自 orval 生成代码） ============
//
// 以下类型由后端 OpenAPI 规范驱动生成，勿手写维护：
// 修改后端 DTO 后执行 `npm run gen:api` 自动同步。

/**
 * 权限枚举（后端 `Permission` 枚举，序列化为变体名如 "Fj200cInformationMonitor"）
 *
 * 注意：这是从 OpenAPI 生成的 **type + const 双导出**（`Permission.Fj200cInformationMonitor`
 * 写法与 TS 枚举用法一致）。成员值由后端 `src/common/models.rs` 驱动，
 * 新增权限点只需改后端并跑 `npm run gen:api`。
 */
export { Permission } from "./api/generated/model";

/** 用户信息（后端 `User` 结构体，password_hash 不返回） */
export type { User } from "./api/generated/model";

/** 登录请求体（POST /api/auth/login） */
export type { LoginRequest } from "./api/generated/model";

/** 登录响应体（JWT token + 用户信息） */
export type { LoginResponse } from "./api/generated/model";

/** 创建用户请求体（POST /api/users） */
export type { CreateUserRequest } from "./api/generated/model";

/** 更新用户角色请求体（PATCH /api/users/:id/role） */
export type { UpdateUserRoleRequest } from "./api/generated/model";

/** 用户设置（后端 `UserSettings` 结构体） */
export type { UserSettings } from "./api/generated/model";
