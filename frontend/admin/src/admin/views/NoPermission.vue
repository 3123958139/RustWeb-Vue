<!--
  管理后台 - 无权限提示页

  当用户角色没有管理后台访问权限时显示。
  提供返回用户端和退出登录两个操作。
-->
<template>
  <div class="no-permission-container">
    <el-result
      icon="warning"
      title="无权限访问"
      sub-title="当前账号没有访问管理后台的权限，请使用管理员账号登录"
    >
      <template #extra>
        <el-button type="primary" @click="goFj200cInformation">返回用户端</el-button>
        <el-button @click="handleLogout">退出登录</el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { useAuthStore } from '@/stores/auth'

/** 路由实例 */
const router = useRouter()
/** 认证 Store */
const authStore = useAuthStore()

/**
 * fj200c_information 应用地址
 * - 开发模式：fj200c_information 的 Vite dev server（固定端口 5173）
 * - 生产环境：后端托管的根路径默认应用
 */
const FJ200C_URL = import.meta.env.DEV ? import.meta.env.VITE_FJ200C_URL ?? 'http://localhost:5173' : '/'

/**
 * 跳转到用户端（fj200c_information 应用）
 */
const goFj200cInformation = () => {
  window.location.href = FJ200C_URL
}

/**
 * 退出登录并跳转到登录页
 *
 * logout() 清除 localStorage 中的 token，
 * router.push() 执行编程式导航
 */
const handleLogout = () => {
  authStore.logout()
  router.push('/login')
  ElMessage.success('已退出登录')
}
</script>

<style scoped>
.no-permission-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background-color: #f5f5f5;
  padding: 16px;
}
</style>
