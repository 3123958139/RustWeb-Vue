<!--
  发动机监控应用（fj200c_information）根组件

  在组件挂载时初始化认证状态，通过 <router-view /> 渲染路由页面。
-->
<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { AppNavbar } from "@shared"
import zhCn from "element-plus/dist/locale/zh-cn.mjs"

/** 路由实例，用于判断当前是否处于登录页 */
const route = useRoute()
/** 登录页不显示导航条，其余页面全局显示 */
const isLoginPage = computed(() => route.path.startsWith('/login'))

/** 认证 Store 实例 */
const authStore = useAuthStore()

/**
 * 组件挂载后初始化认证状态
 * 从 localStorage 恢复 token，并向后端校验有效性
 */
onMounted(() => {
  authStore.initAuth()
})
</script>

<template>
  <!-- 路由出口：根据当前路径渲染对应的页面组件 -->
  <el-config-provider :locale="zhCn">
  <div id="app">
    <!-- 全局导航条（登录页除外），所有角色所有页面共用 -->
    <AppNavbar v-if="!isLoginPage" />
    <router-view />
  </div>
  </el-config-provider>
</template>

<style>
/* 全局样式（非 scoped），作用于整个应用 */
#app {
  font-family: 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', '微软雅黑', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  height: 100vh;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background-color: #f5f5f5;
}
</style>
