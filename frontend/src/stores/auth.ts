import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { User, MenuItem, UserPermissions } from "@/types";
import { Permission, UserRole } from "@/types";
import { apiService } from "@/api";

export const useAuthStore = defineStore("auth", () => {
  const user = ref<User | null>(null);
  const token = ref<string | null>(null);
  const permissions = ref<Permission[]>([]);
  const menuItems = ref<MenuItem[]>([]);

  const isAuthenticated = computed(() => !!token.value && !!user.value);

  // 获取用户角色
  const userRole = computed(() => {
    if (!user.value) return UserRole.User;
    return user.value.role as UserRole;
  });

  // 检查权限
  const hasPermission = (permission: Permission): boolean => {
    return permissions.value.includes(permission);
  };

  // 检查是否有任意一个权限
  const hasAnyPermission = (requiredPermissions: Permission[]): boolean => {
    return requiredPermissions.some((permission) => hasPermission(permission));
  };

  // 检查是否有所有权限
  const hasAllPermissions = (requiredPermissions: Permission[]): boolean => {
    return requiredPermissions.every((permission) => hasPermission(permission));
  };

  // 根据角色获取权限
  const getPermissionsByRole = (role: UserRole): Permission[] => {
    switch (role) {
      case UserRole.Admin:
        return [Permission.Dashboard, Permission.PostsRead, Permission.PostsWrite, Permission.PostsDelete, Permission.UsersRead, Permission.UsersWrite, Permission.UsersDelete, Permission.Settings, Permission.SystemAdmin];
      case UserRole.Moderator:
        return [Permission.Dashboard, Permission.PostsRead, Permission.PostsWrite, Permission.PostsDelete, Permission.UsersRead, Permission.Settings];
      case UserRole.User:
        return [Permission.Dashboard, Permission.PostsRead, Permission.PostsWrite, Permission.Settings];
      default:
        return [];
    }
  };

  // 获取菜单配置
  const getMenuConfig = (): MenuItem[] => {
    const userPerms = permissions.value;

    const allMenus: MenuItem[] = [
      {
        id: "dashboard",
        title: "仪表盘",
        path: "/dashboard",
        icon: "DataBoard",
        permissions: [Permission.Dashboard],
      },
      {
        id: "posts",
        title: "文章管理",
        path: "/posts",
        icon: "Document",
        permissions: [Permission.PostsRead],
        children: [
          {
            id: "posts-list",
            title: "文章列表",
            path: "/posts",
            icon: "List",
            permissions: [Permission.PostsRead],
          },
          {
            id: "posts-create",
            title: "创建文章",
            path: "/posts/create",
            icon: "Plus",
            permissions: [Permission.PostsWrite],
          },
        ],
      },
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
      {
        id: "settings",
        title: "系统设置",
        path: "/settings",
        icon: "Setting",
        permissions: [Permission.Settings],
        children: [
          {
            id: "profile",
            title: "个人资料",
            path: "/profile",
            icon: "UserFilled",
            permissions: [Permission.Settings],
          },
          {
            id: "system-settings",
            title: "系统配置",
            path: "/settings",
            icon: "Setting",
            permissions: [Permission.SystemAdmin],
          },
        ],
      },
    ];

    return filterMenusByPermissions(allMenus, userPerms);
  };

  // 过滤菜单项
  const filterMenusByPermissions = (menus: MenuItem[], userPermissions: Permission[]): MenuItem[] => {
    return menus
      .filter((menu) => {
        // 检查主菜单权限
        return menu.permissions.some((p) => userPermissions.includes(p));
      })
      .map((menu) => {
        // 过滤子菜单
        const filteredChildren = menu.children?.filter((child) => {
          return child.permissions.some((p) => userPermissions.includes(p));
        });

        return {
          ...menu,
          children: filteredChildren && filteredChildren.length > 0 ? filteredChildren : undefined,
        };
      })
      .filter((menu) => {
        // 如果有子菜单但过滤后为空，则不显示主菜单
        if (menu.children && menu.children.length === 0) {
          return false;
        }
        return true;
      });
  };

  // 初始化状态
  const initAuth = () => {
    const savedToken = localStorage.getItem("token");
    const savedUser = localStorage.getItem("user");

    if (savedToken && savedUser) {
      token.value = savedToken;
      user.value = JSON.parse(savedUser);

      // 根据用户角色设置权限
      if (user.value) {
        permissions.value = getPermissionsByRole(user.value.role as UserRole);
        menuItems.value = getMenuConfig();
      }
    }
  };

  // 登录
  const login = async (email: string, password: string) => {
    try {
      const response = await apiService.login({ email, password });
      if (response.success && response.data) {
        token.value = response.data.token;
        user.value = response.data.user;

        localStorage.setItem("token", response.data.token);
        localStorage.setItem("user", JSON.stringify(response.data.user));

        // 设置权限和菜单
        permissions.value = getPermissionsByRole(response.data.user.role as UserRole);
        menuItems.value = getMenuConfig();

        return { success: true };
      } else {
        return { success: false, message: response.message };
      }
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "登录失败",
      };
    }
  };

  // 注册
  const register = async (username: string, email: string, password: string) => {
    try {
      const response = await apiService.register({ username, email, password });
      if (response.success) {
        return { success: true };
      } else {
        return { success: false, message: response.message };
      }
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "注册失败",
      };
    }
  };

  // 登出
  const logout = () => {
    user.value = null;
    token.value = null;
    permissions.value = [];
    menuItems.value = [];
    localStorage.removeItem("token");
    localStorage.removeItem("user");
  };

  // 获取用户信息
  const fetchProfile = async () => {
    try {
      const response = await apiService.getProfile();
      if (response.success && response.data) {
        user.value = response.data;
        localStorage.setItem("user", JSON.stringify(response.data));

        // 更新权限和菜单
        permissions.value = getPermissionsByRole(response.data.role as UserRole);
        menuItems.value = getMenuConfig();
      }
    } catch (error) {
      console.error("获取用户信息失败:", error);
    }
  };

  return {
    user,
    token,
    permissions,
    menuItems,
    isAuthenticated,
    userRole,
    hasPermission,
    hasAnyPermission,
    hasAllPermissions,
    getMenuConfig,
    initAuth,
    login,
    register,
    logout,
    fetchProfile,
  };
});
