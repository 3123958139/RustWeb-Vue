import { defineStore } from "pinia";
import { ref } from "vue";
import type { Post, CreatePostRequest } from "@/types";
import { apiService } from "@/api";

export const usePostsStore = defineStore("posts", () => {
  const posts = ref<Post[]>([]);
  const currentPost = ref<Post | null>(null);
  const loading = ref(false);

  // 获取文章列表
  const fetchPosts = async () => {
    loading.value = true;
    try {
      const response = await apiService.getPosts();
      if (response.success && response.data) {
        posts.value = response.data;
      }
    } catch (error) {
      console.error("获取文章列表失败:", error);
    } finally {
      loading.value = false;
    }
  };

  // 获取单个文章
  const fetchPost = async (id: string) => {
    loading.value = true;
    try {
      const response = await apiService.getPost(id);
      if (response.success && response.data) {
        currentPost.value = response.data;
      }
    } catch (error) {
      console.error("获取文章失败:", error);
    } finally {
      loading.value = false;
    }
  };

  // 创建文章
  const createPost = async (data: CreatePostRequest) => {
    loading.value = true;
    try {
      const response = await apiService.createPost(data);
      if (response.success && response.data) {
        posts.value.unshift(response.data);
        return { success: true, data: response.data };
      } else {
        return { success: false, message: response.message };
      }
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "创建文章失败",
      };
    } finally {
      loading.value = false;
    }
  };

  // 更新文章
  const updatePost = async (id: string, data: CreatePostRequest) => {
    loading.value = true;
    try {
      const response = await apiService.updatePost(id, data);
      if (response.success && response.data) {
        const index = posts.value.findIndex((post) => post.id === id);
        if (index !== -1) {
          posts.value[index] = response.data;
        }
        if (currentPost.value?.id === id) {
          currentPost.value = response.data;
        }
        return { success: true, data: response.data };
      } else {
        return { success: false, message: response.message };
      }
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "更新文章失败",
      };
    } finally {
      loading.value = false;
    }
  };

  // 删除文章
  const deletePost = async (id: string) => {
    loading.value = true;
    try {
      const response = await apiService.deletePost(id);
      if (response.success) {
        posts.value = posts.value.filter((post) => post.id !== id);
        if (currentPost.value?.id === id) {
          currentPost.value = null;
        }
        return { success: true };
      } else {
        return { success: false, message: response.message };
      }
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "删除文章失败",
      };
    } finally {
      loading.value = false;
    }
  };

  return {
    posts,
    currentPost,
    loading,
    fetchPosts,
    fetchPost,
    createPost,
    updatePost,
    deletePost,
  };
});
