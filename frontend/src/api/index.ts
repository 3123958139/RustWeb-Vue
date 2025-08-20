import axios, { AxiosInstance } from "axios";
import type { User, Post, LoginRequest, RegisterRequest, CreatePostRequest, LoginResponse, ApiResponse } from "@/types";

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
}

export const apiService = new ApiService();
