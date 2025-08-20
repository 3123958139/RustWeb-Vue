# 权限控制系统实现文档

## 概述

本项目实现了一个完整的基于角色的权限控制系统（RBAC），包括前端权限控制、动态菜单、路由守卫和后端权限验证。

## 功能特性

### 1. 用户角色系统
- **管理员 (Admin)**: 拥有所有权限
- **版主 (Moderator)**: 拥有文章管理和用户查看权限
- **用户 (User)**: 基础权限，可以管理自己的文章

### 2. 权限枚举
```typescript
enum Permission {
  Dashboard = "dashboard",           // 仪表盘访问
  PostsRead = "posts:read",         // 文章读取
  PostsWrite = "posts:write",       // 文章创建/编辑
  PostsDelete = "posts:delete",     // 文章删除
  UsersRead = "users:read",         // 用户查看
  UsersWrite = "users:write",       // 用户创建/编辑
  UsersDelete = "users:delete",     // 用户删除
  Settings = "settings",            // 设置访问
  SystemAdmin = "system:admin",     // 系统管理
}
```

### 3. 动态菜单系统
- 根据用户权限自动显示/隐藏菜单项
- 支持多级菜单（主菜单 + 子菜单）
- 权限不足时自动隐藏相关功能

### 4. 路由权限控制
- 路由级别的权限验证
- 无权限访问时自动重定向
- 支持多权限要求（任一权限或所有权限）

## 技术实现

### 后端实现

#### 1. 权限模型 (`src/models.rs`)
```rust
// 用户角色枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

// 权限枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    Dashboard,
    PostsRead,
    PostsWrite,
    PostsDelete,
    UsersRead,
    UsersWrite,
    UsersDelete,
    Settings,
    SystemAdmin,
}

// 菜单项结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItem {
    pub id: String,
    pub title: String,
    pub path: String,
    pub icon: String,
    pub permissions: Vec<Permission>,
    pub children: Option<Vec<MenuItem>>,
}
```

#### 2. 权限中间件 (`src/middleware.rs`)
```rust
// 权限验证中间件
pub async fn permission_middleware(
    required_permission: Permission,
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 验证用户权限
    if !user.has_permission(&required_permission) {
        return Err(StatusCode::FORBIDDEN);
    }
    // ...
}

// 角色验证中间件
pub async fn role_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 验证用户角色
    // ...
}
```

#### 3. 权限处理函数 (`src/handlers.rs`)
```rust
// 获取用户权限
pub async fn get_user_permissions(
    State(db): State<DatabaseConnection>,
    user: User,
) -> Result<Json<ApiResponse<UserPermissions>>, StatusCode>

// 获取菜单配置
pub async fn get_menu_config(
    State(_db): State<DatabaseConnection>,
    user: User,
) -> Result<Json<ApiResponse<Vec<MenuItem>>>, StatusCode>

// 检查用户权限
pub async fn check_permission(
    State(_db): State<DatabaseConnection>,
    user: User,
    Json(permission): Json<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode>
```

### 前端实现

#### 1. 权限类型定义 (`frontend/src/types/index.ts`)
```typescript
// 权限枚举
export enum Permission {
  Dashboard = "dashboard",
  PostsRead = "posts:read",
  PostsWrite = "posts:write",
  PostsDelete = "posts:delete",
  UsersRead = "users:read",
  UsersWrite = "users:write",
  UsersDelete = "users:delete",
  Settings = "settings",
  SystemAdmin = "system:admin",
}

// 用户角色枚举
export enum UserRole {
  Admin = "admin",
  Moderator = "moderator",
  User = "user",
}

// 菜单项接口
export interface MenuItem {
  id: string;
  title: string;
  path: string;
  icon: string;
  permissions: Permission[];
  children?: MenuItem[];
}
```

#### 2. 权限Store (`frontend/src/stores/auth.ts`)
```typescript
export const useAuthStore = defineStore("auth", () => {
  const permissions = ref<Permission[]>([]);
  const menuItems = ref<MenuItem[]>([]);

  // 检查权限
  const hasPermission = (permission: Permission): boolean => {
    return permissions.value.includes(permission);
  };

  // 检查是否有任意一个权限
  const hasAnyPermission = (requiredPermissions: Permission[]): boolean => {
    return requiredPermissions.some(permission => hasPermission(permission));
  };

  // 检查是否有所有权限
  const hasAllPermissions = (requiredPermissions: Permission[]): boolean => {
    return requiredPermissions.every(permission => hasPermission(permission));
  };

  // 根据角色获取权限
  const getPermissionsByRole = (role: UserRole): Permission[] => {
    // 根据角色返回对应权限列表
  };

  // 获取菜单配置
  const getMenuConfig = (): MenuItem[] => {
    // 根据用户权限过滤菜单项
  };
});
```

#### 3. 权限控制组件 (`frontend/src/components/PermissionGuard.vue`)
```vue
<template>
  <div v-if="hasPermission">
    <slot />
  </div>
</template>

<script setup lang="ts">
interface Props {
  permission?: Permission;
  permissions?: Permission[];
  requireAll?: boolean;
}

const hasPermission = computed(() => {
  // 根据传入的权限要求进行验证
});
</script>
```

#### 4. 动态导航栏 (`frontend/src/components/AppNavbar.vue`)
```vue
<template>
  <div class="nav-menu">
    <!-- 动态菜单项 -->
    <template v-for="menu in menuItems" :key="menu.id">
      <!-- 没有子菜单的项目 -->
      <router-link 
        v-if="!menu.children || menu.children.length === 0"
        :to="menu.path" 
        class="nav-item" 
        :class="{ active: isActive(menu.path) }"
      >
        <el-icon><component :is="getIcon(menu.icon)" /></el-icon>
        <span>{{ menu.title }}</span>
      </router-link>
      
      <!-- 有子菜单的项目 -->
      <el-dropdown v-else>
        <!-- 子菜单内容 -->
      </el-dropdown>
    </template>
  </div>
</template>
```

#### 5. 路由权限守卫 (`frontend/src/router/index.ts`)
```typescript
// 路由守卫
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore();

  // 检查权限要求
  if (to.meta.permissions) {
    const requiredPermissions = to.meta.permissions as Permission[];
    const hasPermission = requiredPermissions.some(permission => 
      authStore.hasPermission(permission)
    );

    if (!hasPermission) {
      next("/dashboard");
      return;
    }
  }

  next();
});
```

## 使用示例

### 1. 在组件中使用权限控制
```vue
<template>
  <div>
    <!-- 单个权限检查 -->
    <PermissionGuard :permission="Permission.PostsWrite">
      <el-button @click="createPost">创建文章</el-button>
    </PermissionGuard>

    <!-- 多个权限检查（任一权限） -->
    <PermissionGuard :permissions="[Permission.PostsWrite, Permission.PostsDelete]">
      <el-button @click="managePosts">管理文章</el-button>
    </PermissionGuard>

    <!-- 多个权限检查（所有权限） -->
    <PermissionGuard :permissions="[Permission.UsersRead, Permission.UsersWrite]" :require-all="true">
      <el-button @click="manageUsers">用户管理</el-button>
    </PermissionGuard>
  </div>
</template>
```

### 2. 在Store中检查权限
```typescript
const authStore = useAuthStore();

// 检查单个权限
if (authStore.hasPermission(Permission.PostsDelete)) {
  // 执行删除操作
}

// 检查多个权限（任一）
if (authStore.hasAnyPermission([Permission.PostsWrite, Permission.PostsDelete])) {
  // 执行操作
}

// 检查多个权限（所有）
if (authStore.hasAllPermissions([Permission.UsersRead, Permission.UsersWrite])) {
  // 执行操作
}
```

### 3. 路由权限配置
```typescript
{
  path: "/users",
  name: "Users",
  component: () => import("@/views/Users.vue"),
  meta: { 
    requiresAuth: true,
    permissions: [Permission.UsersRead]
  },
}
```

## 权限分配

### 管理员 (Admin)
- 所有权限
- 可以管理用户角色
- 可以访问系统设置

### 版主 (Moderator)
- Dashboard
- PostsRead, PostsWrite, PostsDelete
- UsersRead
- Settings

### 用户 (User)
- Dashboard
- PostsRead, PostsWrite
- Settings

## 安全考虑

1. **前端权限控制**: 提供良好的用户体验，隐藏无权限的功能
2. **后端权限验证**: 确保API级别的安全性
3. **路由守卫**: 防止直接URL访问无权限页面
4. **JWT Token**: 安全的身份验证机制
5. **角色继承**: 高级角色自动拥有低级角色的权限

## 扩展建议

1. **细粒度权限**: 可以进一步细分权限，如按资源类型或操作类型
2. **动态权限**: 支持运行时修改用户权限
3. **权限组**: 支持权限组合和批量管理
4. **审计日志**: 记录权限相关的操作日志
5. **权限缓存**: 优化权限检查的性能

## 文件结构

```
src/
├── models.rs              # 权限相关模型定义
├── middleware.rs          # 权限验证中间件
├── handlers.rs            # 权限相关API处理
└── routes.rs              # 路由配置

frontend/src/
├── types/index.ts         # 权限类型定义
├── stores/auth.ts         # 权限状态管理
├── components/
│   ├── PermissionGuard.vue    # 权限控制组件
│   └── AppNavbar.vue          # 动态导航栏
├── views/
│   └── Users.vue              # 用户管理页面
└── router/index.ts        # 路由权限守卫
```

这个权限控制系统提供了完整的RBAC功能，确保了应用的安全性和用户体验。
