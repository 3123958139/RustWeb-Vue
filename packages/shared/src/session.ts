/**
 * @module session
 * @description 会话管理模块
 *
 * 本模块负责管理用户的登录会话，包括：
 * - 会话的存储（localStorage）
 * - 会话的加载和验证
 * - 旧版会话的迁移
 * - 会话的清除
 *
 * ## 设计原则
 *
 * ### 统一会话键
 * 所有前端应用（fj200c_information、admin、fw150、ftj1c）共享同一个 localStorage 键 `session`。
 * 这确保了：
 * 1. 任何时刻只有一份会话数据
 * 2. 最近一次登录的用户是唯一事实来源
 * 3. 避免"多 key 残留旧用户"的问题
 *
 * ### 旧版迁移
 * 项目早期使用双 key 会话（token/user 用于用户端，admin_token/admin_user 用于管理端）。
 * 本模块会自动检测并迁移到统一会话键，同时清理旧 key。
 *
 * ### 会话结构
 * ```json
 * {
 *   "token": "jwt_token_string",
 *   "user": {
 *     "id": "uuid",
 *     "username": "admin",
 *     "email": "admin@example.com",
 *     "role": "admin",
 *     "created_at": "2024-01-01T00:00:00Z",
 *     "updated_at": "2024-01-01T00:00:00Z"
 *   }
 * }
 * ```
 */

import type {User} from "./types";

/**
 * @constant SESSION_KEY
 * @description localStorage 中存储会话的键名
 *
 * 所有前端应用共享此键，确保会话一致性。
 * 修改此值会影响所有应用的会话读取，需谨慎操作。
 */
export const SESSION_KEY = "session";

/**
 * @interface SessionData
 * @description 会话数据结构
 *
 * 定义存储在 localStorage 中的会话数据格式。
 * 包含 JWT Token 和用户完整信息。
 *
 * 注意：这是内部接口，不对外导出。
 */
interface SessionData {
    /** JWT Token（用于 API 请求的 Authorization 头） */
    token: string;
    /** 用户完整信息（包含角色、权限等） */
    user: User;
}

/**
 * @constant LEGACY_KEYS
 * @description 旧版会话键列表（用于迁移和清理）
 *
 * 项目早期使用的会话键，现在已废弃。
 * 新代码不应使用这些键，仅用于兼容旧版数据迁移。
 *
 * 旧版键名：
 * - `token`: 用户端 token（对应 fj200c_information/fw150/ftj1c 应用）
 * - `user`: 用户端用户信息
 * - `admin_token`: 管理端 token（对应 admin 应用）
 * - `admin_user`: 管理端用户信息
 *
 * `as const` 语法：将数组字面量类型推断为只读元组类型，
 * 确保数组元素不可修改，且类型更精确。
 */
export const LEGACY_KEYS = ["token", "user", "admin_token", "admin_user"] as const;

/**
 * @function loadSession
 * @description 从 localStorage 加载会话数据
 *
 * @returns {SessionData | null} 会话数据，未找到或数据无效返回 null
 *
 * 加载流程：
 * 1. 尝试从 `session` 键读取 JSON 数据
 * 2. 验证数据结构（必须包含 token 和 user.role）
 * 3. 如果失败，尝试从旧版键迁移数据
 * 4. 迁移成功后，清理旧版键，保存到统一会话键
 *
 * 使用场景：
 * - 应用初始化时加载用户会话
 * - 页面刷新后恢复登录状态
 * - 多标签页同步会话状态
 *
 * 数据验证：
 * - token 必须是字符串类型
 * - user 对象必须存在
 * - user.role 必须是字符串类型
 *
 * 错误处理：
 * - JSON 解析失败：静默处理，返回 null
 * - 数据结构无效：静默处理，返回 null
 * - 旧版迁移失败：静默处理，返回 null
 */
export function loadSession(): SessionData | null {
    // 步骤1：尝试从统一会话键读取
    const raw = localStorage.getItem(SESSION_KEY);
    if (raw) {
        try {
            // 解析 JSON 数据
            const parsed = JSON.parse(raw) as SessionData;
            // 验证数据结构
            if (
                parsed &&
                typeof parsed.token === "string" &&
                parsed.user &&
                typeof parsed.user.role === "string"
            ) {
                return parsed;
            }
        } catch {
            // JSON 解析失败，数据损坏，走迁移/清除逻辑
        }
    }

    // 步骤2：尝试从旧版键迁移数据
    // 优先使用用户端键（token/user），其次使用管理端键（admin_token/admin_user）
    const legacyToken = localStorage.getItem("token") || localStorage.getItem("admin_token");
    const legacyUserRaw = localStorage.getItem("user") || localStorage.getItem("admin_user");

    if (legacyToken && legacyUserRaw) {
        try {
            // 解析旧版用户信息
            const legacyUser = JSON.parse(legacyUserRaw);
            // 验证数据结构
            if (legacyUser && typeof legacyUser.role === "string") {
                // 清理所有旧版键
                LEGACY_KEYS.forEach((key) => localStorage.removeItem(key));
                // 构建新会话数据
                const session: SessionData = {token: legacyToken, user: legacyUser};
                // 保存到统一会话键
                localStorage.setItem(SESSION_KEY, JSON.stringify(session));
                return session;
            }
        } catch {
            // 旧版数据损坏，忽略
        }
    }

    // 步骤3：无有效会话
    return null;
}

/**
 * @function saveSession
 * @description 保存会话数据到 localStorage
 *
 * @param {string} token - JWT Token
 * @param {User} user - 用户信息
 *
 * 使用场景：
 * - 登录成功后保存会话
 * - 用户信息更新后同步会话
 * - Token 刷新后更新会话
 *
 * 注意：
 * - 会覆盖已有的会话数据
 * - 数据会序列化为 JSON 字符串存储
 */
export function saveSession(token: string, user: User): void {
    localStorage.setItem(SESSION_KEY, JSON.stringify({token, user}));
}

/**
 * @function clearSession
 * @description 清除所有会话数据
 *
 * 清除范围：
 * - 统一会话键（session）
 * - 所有旧版键（token, user, admin_token, admin_user）
 *
 * 使用场景：
 * - 用户登出
 * - Token 过期或无效
 * - 账户被删除
 * - 安全清理（如检测到异常）
 */
export function clearSession(): void {
    // 清除统一会话键
    localStorage.removeItem(SESSION_KEY);
    // 清除所有旧版键（确保完全清理）
    LEGACY_KEYS.forEach((key) => localStorage.removeItem(key));
}

/**
 * @function getSessionToken
 * @description 从会话中获取 JWT Token
 *
 * @returns {string | null} JWT Token，未找到或无效返回 null
 *
 * 读取优先级：
 * 1. 统一会话键（session）
 * 2. 旧版用户端键（token）
 * 3. 旧版管理端键（admin_token）
 *
 * 使用场景：
 * - API 请求拦截器添加 Authorization 头
 * - 验证用户是否已登录
 *
 * 兼容性：
 * - 支持新旧版会话格式
 * - 自动处理数据损坏情况
 */
export function getSessionToken(): string | null {
    try {
        // 步骤1：尝试从统一会话键读取
        const raw = localStorage.getItem(SESSION_KEY);
        if (raw) {
            const parsed = JSON.parse(raw);
            if (parsed && typeof parsed.token === "string") return parsed.token;
        }
    } catch {
        // 数据损坏，忽略
    }

    // 步骤2：尝试从旧版键读取（兼容旧版）
    // 注意：这里不执行迁移，仅读取
    return localStorage.getItem("token") || localStorage.getItem("admin_token") || null;
}

/**
 * @function buildWebSocketUrl
 * @description 构建带 JWT token 的 WebSocket 地址（浏览器 WS 不支持自定义 header）
 *
 * @param apiPath - WebSocket 端点路径（如 `/api/fj200c_information/ws`）
 * @returns 完整 WebSocket URL（含 token 查询参数）
 *
 * 说明：
 * - 协议跟随当前页面（https 页面用 wss，否则用 ws）
 * - 开发环境由 Vite 代理（/api 含 ws: true），生产环境同源
 */
export function buildWebSocketUrl(apiPath: string): string {
    const token = getSessionToken() || "";
    const protocol = window.location.protocol === "https:" ? "wss" : "ws";
    return `${protocol}://${window.location.host}${apiPath}?token=${encodeURIComponent(token)}`;
}
