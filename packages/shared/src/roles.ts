/**
 * @module roles
 * @description 角色注册表模块
 *
 * 本模块是整个权限系统的核心，定义了所有用户角色及其对应的权限和菜单。
 *
 * ## 架构设计
 *
 * ### 注册表单一来源（orval 同步）
 * - 角色注册表数据（key / name / permissions）由**后端 `src/roles.rs` 唯一维护**
 * - 通过 `GET /api/meta/roles` 暴露，orval 生成 `RoleInfo` 类型与请求函数
 * - 本模块启动时调用 `loadRoleRegistry()` 拉取并缓存，`findRole` / `getPermissionsByRole` /
 *   `getAllRoles` 等函数从缓存读取，不再手写注册表数据
 * - 菜单（userMenus / adminMenus）是纯前端 UI 概念，仍在本文件本地维护（`MENU_CONFIG`）
 *
 * ### 三层架构约定
 * 1. **公共代码层**（本包）：会话管理、类型定义、角色注册表、工厂函数、公共组件
 * 2. **角色模板层**（本包 template/）：可复用的 Vue 组件，无应用依赖
 * 3. **角色应用层**（frontend/*）：每个角色一个独立的前端应用，包含专属 API 和路由
 *
 * ### 新增角色流程
 * 1. 在后端 `src/roles.rs` 的 `ROLE_REGISTRY` 中添加角色定义（key / name / permissions）
 * 2. 后端加 `#[utoipa::path]` 注解的模块路由（tags="xxx"），`src/api_docs.rs` 注册
 * 3. 运行 `npm run gen:api` 重新生成前端类型
 * 4. 在本文件 `MENU_CONFIG` 中添加该角色的菜单配置
 * 5. 复制现有用户端应用（如 `frontend/fw150`）为新应用
 *
 * ### 菜单渲染机制
 * 1. 根据用户角色从 MENU_CONFIG 获取菜单列表
 * 2. 根据用户权限过滤无权限的菜单项（`filterMenusByPermissions`）
 * 3. 根据应用类型（user/admin）选择对应的菜单源
 */

import {type MenuItem, Permission} from "./types";
import {getMeta} from "./api/generated";
import type {RoleInfo} from "./api/generated/model";

/**
 * @interface RoleDef
 * @description 角色定义接口（注册表数据来自后端 `GET /api/meta/roles`）
 *
 * 定义单个角色的完整配置，包括：
 * - 身份标识与权限（key, name, permissions —— 后端同步）
 * - 两类菜单（userMenus, adminMenus —— 前端本地维护）
 */
export interface RoleDef extends RoleInfo {
    /**
     * 用户端应用菜单
     * 定义该角色在自己的专属应用（如 fj200c_information、fw150）中显示的功能面板
     * 这些菜单会显示在角色专属应用的导航栏中
     *
     * 注意：admin 角色通常不需要用户端菜单（管理后台就是它的用户端）
     */
    userMenus: MenuItem[];

    /**
     * 管理端应用菜单
     * 定义该角色在管理后台（admin 应用）中显示的管理功能入口
     * 只有需要在管理后台操作的角色才需要配置此菜单
     *
     * 当前配置：
     * - admin 角色：用户管理（用户列表、创建用户）
     * - 其他角色：空数组（不在管理后台显示）
     */
    adminMenus: MenuItem[];
}

/**
 * 前端本地菜单配置（纯 UI 概念，不参与后端同步）
 *
 * 菜单路径约定：
 * - 用户端应用：`/角色key/功能`（如 `/fj200c_information/monitor`）
 * - 管理端应用：`/功能`（如 `/users`, `/users/create`）
 */
const MENU_CONFIG: Record<string, { userMenus: MenuItem[]; adminMenus: MenuItem[] }> = {
    // ============ 管理员角色 ============
    // 职责：用户管理（增删改查）
    // 应用：管理后台（admin，端口 5174）
    // 特殊：拥有系统管理员权限，可访问所有功能
    admin: {
        userMenus: [], // 管理员没有用户端菜单（管理后台就是它的"用户端"）
        adminMenus: [
            {
                id: "users",
                title: "用户管理",
                path: "/users",
                icon: "User",
                permissions: [Permission.UsersRead],
                children: [
                    {
                        id: "users-list",
                        title: "用户列表",
                        path: "/users",
                        icon: "List",
                        permissions: [Permission.UsersRead],
                    },
                    {
                        id: "users-create",
                        title: "创建用户",
                        path: "/users/create",
                        icon: "Plus",
                        permissions: [Permission.UsersWrite],
                    },
                ],
            },
        ],
    },
    // ============ fj200c_information 角色（发动机监控） ============
    // 职责：实时监控、数据分析、数据记录、配置管理
    // 应用：fj200c_information（端口 5173）
    // 特殊：菜单全部为一级菜单（无子菜单），平铺在导航栏
    fj200c_information: {
        userMenus: [
            {
                id: "fj200c_information-monitor",
                title: "实时监控",
                path: "/fj200c_information/monitor",
                icon: "DataLine",
                permissions: [Permission.Fj200cInformationMonitor],
            },
            {
                id: "fj200c_information-visual",
                title: "可视化分析",
                path: "/fj200c_information/visual",
                icon: "TrendCharts",
                permissions: [Permission.Fj200cInformationMonitor],
            },
            {
                id: "fj200c_information-data",
                title: "数据记录",
                path: "/fj200c_information/data",
                icon: "Histogram",
                permissions: [Permission.Fj200cInformationMonitor],
            },
            {
                id: "fj200c_information-config",
                title: "打开配置",
                path: "/fj200c_information/config",
                icon: "Setting",
                permissions: [Permission.Fj200cInformationMonitor],
            },
            {
                id: "fj200c_information-help",
                title: "帮助",
                path: "/fj200c_information/help",
                icon: "QuestionFilled",
                permissions: [Permission.Fj200cInformationMonitor],
            },
        ],
        adminMenus: [], // fj200c_information 角色不在管理后台显示
    },
    // ============ fj200c_main 角色（发动机测控 ECU/Adam4015/Adam4117/Dyno/Flux 五路串口） ============
    // 职责：五路串口实时测控、试验信息管理、报表生成、CSV 录制
    // 应用：fj200c_main（端口 5179）
    // 特殊：仪表盘维持 1920×1080 设计尺寸 + CSS scale 缩放；
    //       保存数据/模拟运行/主题切换通过 AppNavbar #actions 插槽按钮操作
    fj200c_main: {
        userMenus: [
            {
                id: "fj200c_main-monitor",
                title: "主仪表盘",
                path: "/fj200c_main/monitor",
                icon: "DataLine",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-experiment-input",
                title: "试验信息录入",
                path: "/fj200c_main/experiment-input",
                icon: "EditPen",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-experiment-view",
                title: "试验信息查看",
                path: "/fj200c_main/experiment-view",
                icon: "Document",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-report",
                title: "生成报表",
                path: "/fj200c_main/report",
                icon: "Printer",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-data",
                title: "数据浏览",
                path: "/fj200c_main/data",
                icon: "Histogram",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-config",
                title: "打开配置",
                path: "/fj200c_main/config",
                icon: "Setting",
                permissions: [Permission.Fj200cMainMonitor],
            },
            {
                id: "fj200c_main-help",
                title: "帮助",
                path: "/fj200c_main/help",
                icon: "QuestionFilled",
                permissions: [Permission.Fj200cMainMonitor],
            },
        ],
        adminMenus: [],
    },
    // ============ fw100 角色（设备台账） ============
    // 职责：设备信息管理、台账查询
    // 应用：fw100（端口 5175）
    fw100: {
        userMenus: [
            {
                id: "fw100",
                title: "设备台账",
                path: "/fw100",
                icon: "Files",
                permissions: [Permission.Fw100Monitor],
            },
        ],
        adminMenus: [],
    },
    // ============ fw150 角色（设备台账） ============
    // 职责：设备信息管理、台账查询
    // 应用：fw150（端口 5178）
    fw150: {
        userMenus: [
            {
                id: "fw150",
                title: "设备台账",
                path: "/fw150",
                icon: "Files",
                permissions: [Permission.Fw150Monitor],
            },
        ],
        adminMenus: [],
    },
    // ============ ftj1c 角色（UDP 通信监控） ============
    // 职责：UDP 通信数据监控、日志查看
    // 应用：ftj1c（端口 5176）
    ftj1c: {
        userMenus: [
            {
                id: "ftj1c-monitor",
                title: "通信监控",
                path: "/ftj1c/monitor",
                icon: "Monitor",
                permissions: [Permission.Ftj1cMonitor],
                children: [
                    {
                        id: "ftj1c-udpmonitor",
                        title: "UDP通信监控",
                        path: "/ftj1c/monitor",
                        icon: "Monitor",
                        permissions: [Permission.Ftj1cMonitor],
                    }],
            },
            {
                id: "ftj1c-help",
                title: "帮助",
                path: "/ftj1c/help",
                icon: "QuestionFilled",
                permissions: [Permission.Ftj1cHelp],
            }
        ],
        adminMenus: [],
    },
    // ============ city3d 角色（城市 3D 展示） ============
    // 职责：城市三维数字孪生展示、城市数据管理
    // 应用：city3d（端口 5177）
    city3d: {
        userMenus: [
            {
                id: "city3d-panorama",
                title: "3D 全景",
                path: "/city3d/main",
                icon: "OfficeBuilding",
                permissions: [Permission.City3dView],
            },
            {
                id: "city3d-data",
                title: "数据管理",
                path: "/city3d/data",
                icon: "DataBoard",
                permissions: [Permission.City3dView],
            },
        ],
        adminMenus: [],
    },
    // ============ protocol_generator 角色（通信协议生成） ============
    // 职责：通信协议表编辑、参数表维护、Markdown/Excel 导出、打印
    // 应用：protocol_generator（端口 5180）
    protocol_generator: {
        userMenus: [
            {
                id: "protocol_generator-editor",
                title: "协议编辑",
                path: "/protocol_generator/editor",
                icon: "EditPen",
                permissions: [Permission.ProtocolGeneratorMonitor],
            },
            {
                id: "protocol_generator-csv",
                title: "CSV 参数表",
                path: "/protocol_generator/csv",
                icon: "Files",
                permissions: [Permission.ProtocolGeneratorMonitor],
            },
        ],
        adminMenus: [],
    },
    // ============ qgc 角色（飞控地面站） ============
    // 职责：MAVLink 飞控连接监控、命令控制、地图航点任务规划
    // 应用：qgc（端口 5181）
    qgc: {
        userMenus: [
            {
                id: "qgc-screen",
                title: "显控中心",
                path: "/qgc/screen",
                icon: "Monitor",
                permissions: [Permission.QgcMonitor],
            },
            {
                id: "qgc-map",
                title: "地图与任务",
                path: "/qgc/map",
                icon: "MapLocation",
                permissions: [Permission.QgcMonitor],
            },
            {
                id: "qgc-monitor",
                title: "仪表盘",
                path: "/qgc/monitor",
                icon: "Odometer",
                permissions: [Permission.QgcMonitor],
            },
            {
                id: "qgc-config",
                title: "打开配置",
                path: "/qgc/config",
                icon: "Setting",
                permissions: [Permission.QgcMonitor],
            },
            {
                id: "qgc-help",
                title: "帮助",
                path: "/qgc/help",
                icon: "QuestionFilled",
                permissions: [Permission.QgcMonitor],
            },
        ],
        adminMenus: [],
    },
    // ============ mario 角色（超级马里奥复刻游戏） ============
    // 职责：浏览器内 Canvas 马里奥平台跳跃游戏 + 高分榜
    // 应用：mario（端口 5182）
    mario: {
        userMenus: [
            {
                id: "mario-game",
                title: "开始游戏",
                path: "/mario/main",
                icon: "VideoPlay",
                permissions: [Permission.MarioMonitor],
            },
            {
                id: "mario-rank",
                title: "高分榜",
                path: "/mario/rank",
                icon: "Trophy",
                permissions: [Permission.MarioMonitor],
            },
        ],
        adminMenus: [],
    },
};

/**
 * 注册表缓存（模块级单例）
 * - `null`：尚未拉取（未初始化）
 * - `RoleInfo[]`：已拉取成功（或失败时为空数组，避免重复请求）
 */
let registryCache: RoleInfo[] | null = null;

/**
 * @function loadRoleRegistry
 * @description 从后端拉取角色注册表并缓存（幂等：仅首次发起请求）
 *
 * 返回 key / name / permissions 的完整角色列表，数据源为后端
 * `src/roles.rs`（通过 `GET /api/meta/roles` 暴露）。
 *
 * 调用时机：
 * - `createAuthStore` 的 `initAuth()`（所有应用路由守卫会先 await 它）
 * - `login()`（登录成功后立即生效）
 *
 * 失败时缓存为空数组（不抛出），不影响登录流程；权限校验始终在后端进行。
 */
export async function loadRoleRegistry(): Promise<RoleInfo[]> {
    if (registryCache) return registryCache;
    try {
        const response = await getMeta().metaListRoles();
        registryCache = response.data ?? [];
    } catch {
        registryCache = [];
    }
    return registryCache;
}

/**
 * @function findRole
 * @description 根据角色 key 查找角色定义（需先 `loadRoleRegistry`）
 *
 * @param {string} roleKey - 角色唯一标识符（如 "admin", "fj200c_information"）
 * @returns {RoleInfo | undefined} 找到的角色定义，未找到返回 undefined
 *
 * 使用示例：
 * ```typescript
 * const role = findRole("fj200c_information");
 * if (role) {
 *   console.log(role.name); // "fj200c_information"
 * }
 * ```
 */
export function findRole(roleKey: string): RoleInfo | undefined {
    // 使用 Array.find() 查找第一个匹配的元素
    return getRoleRegistry().find((r) => r.key === roleKey);
}

/**
 * @function isRegisteredRole
 * @description 检查角色是否已注册（需先 `loadRoleRegistry`）
 *
 * @param {string} roleKey - 角色唯一标识符
 * @returns {boolean} 角色已注册返回 true，否则返回 false
 *
 * 使用场景：
 * - 路由守卫：验证用户角色是否有效
 * - 登录验证：确认角色存在后才允许登录
 */
export function isRegisteredRole(roleKey: string): boolean {
    // 使用 Array.some() 检查是否存在匹配的元素
    return getRoleRegistry().some((r) => r.key === roleKey);
}

/**
 * @function getRoleRegistry
 * @description 获取已缓存的注册表（未加载时返回空数组）
 *
 * 同步读取，供注册表驱动函数（findRole / getPermissionsByRole 等）内部使用。
 * 业务代码请先 `await loadRoleRegistry()` 再调用。
 */
export function getRoleRegistry(): RoleInfo[] {
    return registryCache ?? [];
}

/**
 * @function getPermissionsByRole
 * @description 获取角色的权限列表（需先 `loadRoleRegistry`）
 *
 * @param {string} roleKey - 角色唯一标识符
 * @returns {Permission[]} 角色拥有的权限点数组，未找到角色返回空数组
 *
 * 权限用途：
 * - 前端：动态生成菜单、控制按钮显隐
 * - 后端：API 访问控制、数据过滤
 */
export function getPermissionsByRole(roleKey: string): Permission[] {
    // 使用可选链（?.）和空值合并（??）操作符
    // ?. 如果 findRole 返回 undefined，则不会访问 .permissions
    // ?? 如果结果是 undefined 或 null，则使用空数组
    return findRole(roleKey)?.permissions ?? [];
}

/**
 * @function getMenusByRole
 * @description 根据角色和应用类型获取菜单列表
 *
 * @param {string} roleKey - 角色唯一标识符
 * @param {"user" | "admin"} appKind - 应用类型
 *   - "user": 用户端应用（角色专属应用，如 fj200c_information、fw150）
 *   - "admin": 管理端应用（admin 应用）
 * @returns {MenuItem[]} 过滤后的菜单列表
 *
 * 菜单过滤逻辑：
 * 1. 根据 appKind 从本地 MENU_CONFIG 选择 userMenus 或 adminMenus
 * 2. 使用 filterMenusByPermissions 过滤无权限的菜单
 */
export function getMenusByRole(roleKey: string, appKind: "user" | "admin"): MenuItem[] {
    const menuConfig = MENU_CONFIG[roleKey];
    if (!menuConfig) return [];

    // 根据应用类型选择对应的菜单源
    const menus = appKind === "admin" ? menuConfig.adminMenus : menuConfig.userMenus;
    // 获取角色权限
    const permissions = getPermissionsByRole(roleKey);
    // 过滤菜单
    return filterMenusByPermissions(menus, permissions);
}

/**
 * @function getAllRoles
 * @description 获取所有已注册角色的简要信息（需先 `loadRoleRegistry`）
 *
 * @returns {Array<{key: string, name: string}>} 角色 key 和名称数组
 *
 * 使用场景：
 * - 管理后台的角色下拉框
 * - 角色选择器组件
 *
 * 返回格式示例：
 * ```json
 * [
 *   { "key": "admin", "name": "管理员" },
 *   { "key": "fj200c_information", "name": "fj200c_information" },
 *   { "key": "fw150", "name": "fw150" },
 *   { "key": "ftj1c", "name": "ftj1c" }
 * ]
 * ```
 */
export function getAllRoles(): { key: string; name: string }[] {
    // 使用 Array.map() 提取 key 和 name 字段
    return getRoleRegistry().map((r) => ({key: r.key, name: r.name}));
}

// ============ 角色 → 所属应用跳转映射 ============
/**
 * @constant ROLE_APP_URLS
 * @description 角色与应用地址的映射关系
 *
 * 定义每个角色对应的应用访问地址，包含开发环境和生产环境两种格式。
 *
 * 使用场景：
 * - 登录成功后跳转到对应角色的应用
 * - 用户在错误的应用登录时自动跳转
 * - 跨应用跳转时获取目标地址
 *
 * 地址格式：
 * - dev: 开发环境完整 URL（如 http://localhost:5173）
 * - prod: 生产环境路径前缀（如 /fj200c_information）
 *
 * 新增角色时需在此映射中添加对应条目。
 */
export const ROLE_APP_URLS: Record<string, { dev: string; prod: string }> = {
    // 管理员 → 管理后台（端口 5174，路径 /admin）
    admin: {dev: "http://localhost:5174", prod: "/admin"},
    // 发动机监控 → fj200c_information（端口 5173，路径 /fj200c_information）
    fj200c_information: {dev: "http://localhost:5173", prod: "/fj200c_information"},
    // 发动机测控 → fj200c_main（端口 5179，路径 /fj200c_main）
    fj200c_main: {dev: "http://localhost:5179", prod: "/fj200c_main"},
    // 设备台账 → fw100（端口 5175，路径 /fw100）
    fw100: {dev: "http://localhost:5175", prod: "/fw100"},
    fw150: {dev: "http://localhost:5178", prod: "/fw150"},
    // UDP 通信监控 → ftj1c（端口 5176，路径 /ftj1c）
    ftj1c: {dev: "http://localhost:5176", prod: "/ftj1c"},
    // 城市 3D 展示 → city3d（端口 5177，路径 /city3d）
    city3d: {dev: "http://localhost:5177", prod: "/city3d"},
    // 通信协议生成 → protocol_generator（端口 5180，路径 /protocol_generator）
    protocol_generator: {dev: "http://localhost:5180", prod: "/protocol_generator"},
    // 飞控地面站 → qgc（端口 5181，路径 /qgc）
    qgc: {dev: "http://localhost:5181", prod: "/qgc"},
    // 超级马里奥复刻游戏 → mario（端口 5182，路径 /mario）
    mario: {dev: "http://localhost:5182", prod: "/mario"},
};

/**
 * @function getRoleAppUrl
 * @description 获取角色所属应用的访问地址
 *
 * @param {string} roleKey - 角色唯一标识符
 * @param {boolean} isDev - 是否为开发环境（true=开发，false=生产）
 * @returns {string | null} 应用访问地址，未登记返回 null
 *
 * 使用示例：
 * ```typescript
 * // 开发环境
 * const devUrl = getRoleAppUrl("fj200c_information", true);
 * // 返回 "http://localhost:5173"
 *
 * // 生产环境
 * const prodUrl = getRoleAppUrl("fj200c_information", false);
 * // 返回 "/fj200c_information"
 * ```
 */
export function getRoleAppUrl(roleKey: string, isDev: boolean): string | null {
    const target = ROLE_APP_URLS[roleKey];
    if (!target) return null;
    // 根据环境返回对应的地址格式
    return isDev ? target.dev : target.prod;
}

/**
 * @function filterMenusByPermissions
 * @description 根据用户权限过滤菜单项（内部函数）
 *
 * @param {MenuItem[]} menus - 原始菜单列表
 * @param {Permission[]} userPermissions - 用户拥有的权限点列表
 * @returns {MenuItem[]} 过滤后的菜单列表
 *
 * 过滤规则：
 * 1. 父菜单：至少有一个权限点在用户权限列表中
 * 2. 子菜单：至少有一个权限点在用户权限列表中
 * 3. 如果父菜单的子菜单全部被过滤，则父菜单也不显示
 * 4. 如果父菜单没有子菜单字段（undefined），则直接显示
 *
 * 注意：这是模块内部函数，不对外导出。
 */
function filterMenusByPermissions(menus: MenuItem[], userPermissions: Permission[]): MenuItem[] {
    return menus
        // 步骤1：过滤父菜单（至少有一个权限匹配）
        .filter((menu) => menu.permissions.some((p) => userPermissions.includes(p)))
        // 步骤2：处理每个菜单的子菜单
        .map((menu) => {
            // 过滤子菜单（至少有一个权限匹配）
            const filteredChildren = menu.children?.filter((child) =>
                child.permissions.some((p) => userPermissions.includes(p))
            );
            // 返回新菜单对象（不可变更新）
            return {
                ...menu, // 展开原菜单属性
                // 如果有子菜单且过滤后不为空，则使用过滤后的子菜单；否则设为 undefined
                children: filteredChildren && filteredChildren.length > 0 ? filteredChildren : undefined,
            };
        })
        // 步骤3：过滤掉"有 children 字段但数组为空"的菜单
        // 这种菜单是父菜单，但所有子菜单都被过滤了，不应显示
        .filter((menu) => !(menu.children && menu.children.length === 0));
}
