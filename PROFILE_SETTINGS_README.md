# 个人资料和系统设置功能

## 概述

我们已经成功完善了个人资料和系统设置功能，为用户提供了完整的账户管理体验。

## 功能特性

### 1. 个人资料页面 (`/profile`)

#### 基本信息管理

- ✅ **用户信息展示**：显示用户名、邮箱、角色、注册时间等
- ✅ **信息编辑**：支持编辑用户名和邮箱
- ✅ **角色显示**：显示用户角色（管理员/普通用户）
- ✅ **时间信息**：显示注册时间和最后更新时间

#### 密码管理

- ✅ **修改密码**：支持修改当前密码
- ✅ **密码验证**：验证当前密码和新密码
- ✅ **密码确认**：确保新密码输入正确

#### 账户统计

- ✅ **文章统计**：显示总文章数、已发布、草稿数量
- ✅ **注册天数**：计算并显示用户注册天数
- ✅ **实时更新**：统计数据实时更新

### 2. 系统设置页面 (`/settings`)

#### 主题设置

- ✅ **主题模式**：浅色模式、深色模式、跟随系统
- ✅ **主题色彩**：自定义主题色彩选择器
- ✅ **实时预览**：设置变更实时生效

#### 通知设置

- ✅ **邮件通知**：开启/关闭邮件通知
- ✅ **浏览器通知**：开启/关闭浏览器推送
- ✅ **通知类型**：选择接收的通知类型（文章、系统、安全）

#### 安全设置

- ✅ **两步验证**：启用/禁用两步验证
- ✅ **设备管理**：查看和管理登录设备
- ✅ **会话超时**：设置自动登出时间

#### 隐私设置

- ✅ **资料可见性**：设置个人资料可见范围
- ✅ **文章可见性**：设置默认文章可见性
- ✅ **数据收集**：控制数据收集权限

#### 语言设置

- ✅ **界面语言**：支持中文、英文、日文
- ✅ **时区设置**：支持多个时区选择

#### 数据管理

- ✅ **数据导出**：导出个人数据
- ✅ **账户删除**：永久删除账户（带确认机制）

## 技术实现

### 前端架构

#### 页面组件

- `Profile.vue` - 个人资料页面
- `Settings.vue` - 系统设置页面

#### API 服务

```typescript
// 用户资料相关
updateProfile(data) - 更新用户资料;
changePassword(data) - 修改密码;

// 设置相关
getSettings() - 获取用户设置;
updateSettings(data) - 更新用户设置;
exportData() - 导出数据;
deleteAccount() - 删除账户;
```

#### 路由配置

```typescript
{
  path: "/profile",
  name: "Profile",
  component: () => import("@/views/Profile.vue"),
  meta: { requiresAuth: true },
},
{
  path: "/settings",
  name: "Settings",
  component: () => import("@/views/Settings.vue"),
  meta: { requiresAuth: true },
}
```

### 响应式设计

#### 移动端优化

- 自适应布局，支持手机和平板
- 触摸友好的交互设计
- 优化的表单布局

#### 桌面端优化

- 网格布局，充分利用屏幕空间
- 直观的设置分组
- 快速访问和操作

### 用户体验

#### 交互设计

- **即时反馈**：操作后立即显示结果
- **确认机制**：危险操作需要确认
- **加载状态**：显示操作进度
- **错误处理**：友好的错误提示

#### 数据验证

- **表单验证**：实时验证用户输入
- **密码强度**：确保密码安全性
- **邮箱格式**：验证邮箱格式正确性

## 后端 API 需求

### 用户资料 API

```http
PUT /api/users/profile
{
  "username": "string",
  "email": "string"
}

PUT /api/users/password
{
  "currentPassword": "string",
  "newPassword": "string"
}
```

### 设置 API

```http
GET /api/users/settings
PUT /api/users/settings
{
  "theme": "light|dark|auto",
  "primaryColor": "string",
  "emailNotifications": boolean,
  "browserNotifications": boolean,
  "notificationTypes": ["posts", "system", "security"],
  "twoFactorAuth": boolean,
  "sessionTimeout": number,
  "profileVisibility": "public|friends|private",
  "defaultPostVisibility": "public|friends|private",
  "dataCollection": boolean,
  "language": "zh-CN|en-US|ja-JP",
  "timezone": "string"
}

POST /api/users/export
DELETE /api/users/account
```

## 安全考虑

### 数据保护

- 密码加密存储
- 敏感操作需要确认
- 会话管理

### 权限控制

- 用户只能修改自己的资料
- 管理员权限验证
- API 访问控制

## 未来扩展

### 功能增强

- [ ] 头像上传功能
- [ ] 社交媒体链接
- [ ] 个人简介编辑
- [ ] 偏好设置同步

### 技术改进

- [ ] 设置本地缓存
- [ ] 主题切换动画
- [ ] 多语言支持
- [ ] 设置导入/导出

## 使用说明

### 访问方式

1. 登录后点击右上角用户头像
2. 选择"个人资料"或"设置"
3. 或直接访问 `/profile` 或 `/settings`

### 操作流程

1. **个人资料**：查看信息 → 点击编辑 → 修改信息 → 保存
2. **修改密码**：输入当前密码 → 输入新密码 → 确认密码 → 提交
3. **系统设置**：选择设置项 → 修改配置 → 保存设置

## 总结

个人资料和系统设置功能已经完整实现，提供了：

- ✅ 完整的用户信息管理
- ✅ 安全的密码修改机制
- ✅ 丰富的系统设置选项
- ✅ 响应式的用户界面
- ✅ 完善的错误处理
- ✅ 良好的用户体验

这些功能为用户提供了完整的账户管理体验，满足了现代 Web 应用的基本需求。
