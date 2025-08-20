import axios, { AxiosInstance } from "axios";
import type { User, Post, LoginRequest, RegisterRequest, CreatePostRequest, LoginResponse, ApiResponse } from "@/types";

// 设置相关类型定义
export interface UserSettings {
  id: string;
  user_id: string;
  theme: string;
  primary_color: string;
  email_notifications: boolean;
  browser_notifications: boolean;
  notification_types: string[];
  two_factor_auth: boolean;
  session_timeout: number;
  profile_visibility: string;
  default_post_visibility: string;
  data_collection: boolean;
  language: string;
  timezone: string;
  created_at: string;
  updated_at: string;
}

export interface UserDevice {
  id: string;
  user_id: string;
  device_name: string;
  browser: string;
  location: string;
  ip_address: string;
  user_agent: string;
  last_login: string;
  is_active: boolean;
  created_at: string;
}

export interface UpdateSettingsRequest {
  theme?: string;
  primary_color?: string;
  email_notifications?: boolean;
  browser_notifications?: boolean;
  notification_types?: string[];
  two_factor_auth?: boolean;
  session_timeout?: number;
  profile_visibility?: string;
  default_post_visibility?: string;
  data_collection?: boolean;
  language?: string;
  timezone?: string;
}

export interface ExportDataRequest {
  email: string;
  data_types: string[];
}

export interface DeleteAccountRequest {
  password: string;
  confirmation: string;
}

class ApiService {
  private api: AxiosInstance;

  constructor() {
    this.api = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || "/api",
      timeout: 10000,
    });

    // 请求拦截器
    this.api.interceptors.request.use(
      (config) => {
        const token = localStorage.getItem("token");
        if (token) {
          config.headers.Authorization = `Bearer ${token}`;
        }
        return config;
      },
      (error) => {
        return Promise.reject(error);
      }
    );

    // 响应拦截器
    this.api.interceptors.response.use(
      (response) => {
        return response;
      },
      (error) => {
        if (error.response?.status === 401) {
          localStorage.removeItem("token");
          localStorage.removeItem("user");
          window.location.href = "/login";
        }
        return Promise.reject(error);
      }
    );
  }

  // 用户相关API
  async register(data: RegisterRequest): Promise<ApiResponse<User>> {
    const response = await this.api.post("/users/register", data);
    return response.data;
  }

  async login(data: LoginRequest): Promise<ApiResponse<LoginResponse>> {
    const response = await this.api.post("/users/login", data);
    return response.data;
  }

  async getProfile(): Promise<ApiResponse<User>> {
    const response = await this.api.get("/users/profile");
    return response.data;
  }

  // 文章相关API
  async getPosts(): Promise<ApiResponse<Post[]>> {
    const response = await this.api.get("/posts");
    return response.data;
  }

  async getPost(id: string): Promise<ApiResponse<Post>> {
    const response = await this.api.get(`/posts/${id}`);
    return response.data;
  }

  async createPost(data: CreatePostRequest): Promise<ApiResponse<Post>> {
    const response = await this.api.post("/posts", data);
    return response.data;
  }

  async updatePost(id: string, data: CreatePostRequest): Promise<ApiResponse<Post>> {
    const response = await this.api.put(`/posts/${id}`, data);
    return response.data;
  }

  async deletePost(id: string): Promise<ApiResponse<void>> {
    const response = await this.api.delete(`/posts/${id}`);
    return response.data;
  }

  // 用户资料相关API
  async updateProfile(data: any): Promise<ApiResponse<User>> {
    const response = await this.api.put("/users/profile", data);
    return response.data;
  }

  async changePassword(data: any): Promise<ApiResponse<void>> {
    const response = await this.api.put("/users/password", data);
    return response.data;
  }

  // 设置相关API
  async getSettings(): Promise<ApiResponse<UserSettings>> {
    const response = await this.api.get("/users/settings");
    return response.data;
  }

  async updateSettings(data: UpdateSettingsRequest): Promise<ApiResponse<UserSettings>> {
    const response = await this.api.put("/users/settings", data);
    return response.data;
  }

  async getUserDevices(): Promise<ApiResponse<UserDevice[]>> {
    const response = await this.api.get("/users/devices");
    return response.data;
  }

  async logoutDevice(deviceId: string): Promise<ApiResponse<void>> {
    const response = await this.api.delete(`/users/devices/${deviceId}`);
    return response.data;
  }

  async exportData(data: ExportDataRequest): Promise<ApiResponse<void>> {
    const response = await this.api.post("/users/export", data);
    return response.data;
  }

  async deleteAccount(data: DeleteAccountRequest): Promise<ApiResponse<void>> {
    const response = await this.api.delete("/users/account", { data });
    return response.data;
  }
}

export const apiService = new ApiService();
