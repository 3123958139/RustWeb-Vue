export interface User {
  id: string;
  username: string;
  email: string;
  role: string;
  created_at: string;
  updated_at: string;
}

export interface Post {
  id: string;
  title: string;
  content: string;
  author_id: string;
  status: string;
  created_at: string;
  updated_at: string;
}

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

// 用户权限接口
export interface UserPermissions {
  user_id: string;
  role: UserRole;
  permissions: Permission[];
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
}

export interface CreatePostRequest {
  title: string;
  content: string;
  status?: string;
}

export interface LoginResponse {
  token: string;
  user: User;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}
