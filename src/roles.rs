//! # 角色注册表模块
//!
//! 本模块是全系统角色定义的**单一事实来源**（Single Source of Truth）。
//!
//! # 架构说明
//!
//! 角色注册表采用**注册表模式**（Registry Pattern）：
//! - 后端：`src/roles.rs` 定义角色和权限
//! - 前端：`packages/shared/src/roles.ts` 定义角色、权限和菜单
//! - 两端通过相同的 `key` 保持同步
//!
//! # 角色定义
//!
//! | 角色 | 说明 | 权限 |
//! |------|------|------|
//! | `admin` | 管理员 | UsersRead, UsersWrite, UsersDelete, SystemAdmin |
//! | `fj200c_information` | 发动机监控 | Fj200cInformationMonitor |
//! | `fw100` | 设备台账 | Fw100Monitor |
//! | `ftj1c` | UDP 通信监控 | Ftj1cMonitor, Ftj1cHelp |
//! | `city3d` | 城市 3D 展示 | City3dView |
//! | `fw150` | 设备台账 | Fw150Monitor |
//! | `fj200c_main` | 发动机测控 | Fj200cMainMonitor |
//! | `protocol_generator` | 通信协议生成 | ProtocolGeneratorMonitor |
//!
//! # 新增角色步骤（后端侧）
//!
//! 1. 在 `ROLE_REGISTRY` 中追加一个 `RoleDef`
//!    - `key` 必须与前端 `packages/shared/src/roles.ts` 的 `key` 一致
//!    - `permissions` 定义该角色拥有的权限
//!
//! 2. 复制 `src/role_template/` 为新角色的功能模块
//!    - 按模板注释挂载路由
//!    - 实现业务逻辑
//!
//! 3. 在路由中为新模块选择中间件
//!    - 普通面板：`permission_middleware`（检查特定权限）
//!    - 管理功能：`role_middleware`（检查 `SystemAdmin` 权限）
//!
//! # 权限系统
//!
//! 本系统采用**基于角色的访问控制**（RBAC, Role-Based Access Control）：
//!
//! ```text
//! 用户 → 角色 → 权限
//! ```
//!
//! - 每个用户有一个角色（`users.role` 字段）
//! - 每个角色有一组权限（本注册表定义）
//! - 中间件检查用户是否拥有所需权限
//!
//! # 为什么需要注册表？
//!
//! 1. **避免硬编码**：角色和权限集中管理，不散落在代码各处
//! 2. **类型安全**：编译时检查权限是否存在
//! 3. **易于维护**：新增角色只需修改注册表
//! 4. **前后端同步**：两端共享同一份角色定义

use crate::common::models::{ApiResponse, Permission};
use axum::Json;
use serde::Serialize;
// 权限枚举类型

/// 角色定义结构体
///
/// 每个角色包含：
/// - `key`: 角色标识（存储在数据库 `users.role` 字段）
/// - `name`: 角色显示名称（用于前端 UI 展示）
/// - `permissions`: 该角色拥有的权限列表
///
/// # 示例
///
/// ```rust
/// let role = RoleDef {
///     key: "admin",
///     name: "管理员",
///     permissions: &[Permission::SystemAdmin],
/// };
/// ```
#[derive(Debug, Clone, Copy)] // 自动派生调试、克隆、复制 trait
pub struct RoleDef {
    /// 数据库 `users.role` 中存储的角色标识
    /// 例如：`"admin"`、`"fj200c_information"`、`"fw150"`、`"ftj1c"`
    pub key: &'static str, // `&'static str` 是静态字符串引用，生命周期为整个程序

    /// 角色显示名称（前端导航栏 / 角色下拉框展示用）
    pub name: &'static str,

    /// 该角色拥有的权限点
    /// 使用 `&'static [Permission]` 静态数组，编译时确定
    /// 例如：`&[Permission::UsersRead, Permission::UsersWrite]`
    pub permissions: &'static [Permission],
}

/// 角色注册表（静态常量）
///
/// `static` 关键字定义全局变量，在程序整个生命周期有效
/// `&'static` 表示引用的生命周期为静态（整个程序运行期间）
///
/// # 设计决策
///
/// 使用 `&[RoleDef]` 而不是 `Vec<RoleDef>`：
/// - 编译时确定大小，性能更好
/// - 不可变，线程安全
/// - 无需堆分配
pub static ROLE_REGISTRY: &[RoleDef] = &[
    // 管理员角色：拥有所有用户管理权限
    RoleDef {
        key: "admin",
        name: "管理员",
        permissions: &[
            Permission::UsersRead,   // 读取用户
            Permission::UsersWrite,  // 创建/更新用户
            Permission::UsersDelete, // 删除用户
            Permission::SystemAdmin, // 系统管理标志（拥有此权限视为管理员）
        ],
    },
    // fj200c_information 角色：发动机监控面板
    RoleDef {
        key: "fj200c_information",
        name: "fj200c_information",
        permissions: &[Permission::Fj200cInformationMonitor], // 发动机监控权限
    },
    // fw100 角色：设备台账面板
    RoleDef {
        key: "fw100",
        name: "fw100",
        permissions: &[Permission::Fw100Monitor], // 设备台账权限
    },
    // ftj1c 角色：UDP 组播通信监控面板
    RoleDef {
        key: "ftj1c",
        name: "ftj1c",
        permissions: &[Permission::Ftj1cMonitor, Permission::Ftj1cHelp], // 通信监控权限
    },
    // city3d 角色：城市 3D 展示面板
    RoleDef {
        key: "city3d",
        name: "city3d",
        permissions: &[Permission::City3dView], // 城市 3D 展示权限
    },
    // fw150 角色：设备台账面板
    RoleDef {
        key: "fw150",
        name: "fw150",
        permissions: &[Permission::Fw150Monitor], // 设备台账权限
    },
    // fj200c_main 角色：发动机测控面板
    RoleDef {
        key: "fj200c_main",
        name: "fj200c_main",
        permissions: &[Permission::Fj200cMainMonitor],
    },
    // protocol_generator 角色：通信协议生成面板
    RoleDef {
        key: "protocol_generator",
        name: "protocol_generator",
        permissions: &[Permission::ProtocolGeneratorMonitor],
    },
];

/// 按 key 查找角色定义
///
/// # 参数
/// - `key`: 角色标识（如 `"admin"`、`"fj200c_information"`）
///
/// # 返回值
/// - `Some(&RoleDef)` - 找到角色定义
/// - `None` - 角色未注册
///
/// # 示例
///
/// ```rust
/// if let Some(role) = find_role("admin") {
///     println!("管理员权限: {:?}", role.permissions);
/// }
/// ```
///
/// # 语法说明
///
/// `Option<&'static RoleDef>` 是 Rust 的枚举类型：
/// - `Some(value)` - 包含值
/// - `None` - 不包含值
///
/// `iter().find()` 是迭代器方法：
/// - `iter()` 创建迭代器
/// - `find()` 查找第一个满足条件的元素
/// - `|r| r.key == key` 是闭包（closure），类似箭头函数
pub fn find_role(key: &str) -> Option<&'static RoleDef> {
    ROLE_REGISTRY.iter().find(|r| r.key == key)
}

/// 检查角色是否已注册
///
/// # 参数
/// - `key`: 角色标识
///
/// # 返回值
/// - `true` - 角色已注册
/// - `false` - 角色未注册
///
/// # 用途
///
/// 管理员创建/修改用户时，验证角色是否有效
/// 例如：`is_registered_role("admin")` 返回 `true`
pub fn is_registered_role(key: &str) -> bool {
    // `find_role().is_some()` 将 `Option` 转换为 `bool`
    // `Some(_)` → `true`，`None` → `false`
    find_role(key).is_some()
}

/// 获取角色对应的权限列表
///
/// # 参数
/// - `key`: 角色标识
///
/// # 返回值
/// - 角色拥有的权限列表
/// - 未知角色返回空列表
///
/// # 示例
///
/// ```rust
/// let perms = permissions_for("admin");
/// assert!(perms.contains(&Permission::SystemAdmin));
/// ```
///
/// # 语法说明
///
/// `map()` 和 `unwrap_or_default()` 是链式调用：
/// - `find_role(key)` → `Option<&RoleDef>`
/// - `.map(|r| r.permissions.to_vec())` → `Option<Vec<Permission>>`
/// - `.unwrap_or_default()` → `Vec<Permission>`（空列表）
pub fn permissions_for(key: &str) -> Vec<Permission> {
    find_role(key)
        .map(|r| r.permissions.to_vec()) // 将 `&[Permission]` 转换为 `Vec<Permission>`
        .unwrap_or_default() // 如果是 `None`，返回空 `Vec`
}

/// 角色信息 DTO（通过 API 暴露给前端，前端注册表由 orval 生成类型后单一同步）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RoleInfo {
    /// 角色标识（数据库 `users.role` 存储值）
    pub key: String,
    /// 角色显示名称
    pub name: String,
    /// 角色拥有的权限点列表
    pub permissions: Vec<Permission>,
}

/// 将注册表转换为可序列化的 `RoleInfo` 列表
pub fn registry_info() -> Vec<RoleInfo> {
    ROLE_REGISTRY
        .iter()
        .map(|r| RoleInfo {
            key: r.key.to_string(),
            name: r.name.to_string(),
            permissions: r.permissions.to_vec(),
        })
        .collect()
}

/// 角色注册表查询（公开端点，前端启动时拉取 key/name/permissions）
///
/// 前端 `packages/shared/src/roles.ts` 不再手写注册表数据，
/// key/name/permissions 全部以本接口返回值为唯一来源。
#[utoipa::path(
    tag = "meta",
    get,
    path = "/api/meta/roles",
    operation_id = "metaListRoles",
    responses(
        (status = 200, description = "角色注册表列表", body = ApiResponse<Vec<RoleInfo>>),
    ),
)]
pub async fn list_roles() -> Json<ApiResponse<Vec<RoleInfo>>> {
    Json(ApiResponse::success(registry_info()))
}
