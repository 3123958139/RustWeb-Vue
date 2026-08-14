<!--
  App.vue —— 前端应用（city3d）根组件

  职责：
  - 登录页不显示顶部导航栏（AppNavbar），其余页面显示
  - 挂载时调用 authStore.initAuth() 恢复登录态并加载角色注册表
  - 通过 el-config-provider 注入 Element Plus 中文语言包
-->
<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { AppNavbar } from '@shared'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'

const route = useRoute()
const isLoginPage = computed(() => route.path.startsWith('/login'))

const authStore = useAuthStore()

onMounted(() => {
  authStore.initAuth()
})
</script>

<template>
  <el-config-provider :locale="zhCn">
  <div id="app">
    <AppNavbar v-if="!isLoginPage" />
    <router-view />
  </div>
  </el-config-provider>
</template>

<style>
#app {
  font-family: 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', '微软雅黑', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  height: 100vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background-color: #0a0e1a;
  color: #e0e0e0;
}
</style>