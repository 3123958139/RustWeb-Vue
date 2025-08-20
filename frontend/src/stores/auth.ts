import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { User } from "@/types";
import { apiService } from "@/api";

export const useAuthStore = defineStore("auth", () => {
  const user = ref<User | null>(null);
  const token = ref<string | null>(null);

  const isAuthenticated = computed(() => !!token.value && !!user.value);

  // 初始化状态
  const initAuth = () => {
    const savedToken = localStorage.getItem("token");
    const savedUser = localStorage.getItem("user");

    if (savedToken && savedUser) {
      token.value = savedToken;
      user.value = JSON.parse(savedUser);
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
      }
    } catch (error) {
      console.error("获取用户信息失败:", error);
    }
  };

  return {
    user,
    token,
    isAuthenticated,
    initAuth,
    login,
    register,
    logout,
    fetchProfile,
  };
});
