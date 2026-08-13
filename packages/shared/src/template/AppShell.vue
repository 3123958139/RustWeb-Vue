<!--
  通用应用外壳（共享组件，6 个应用复用）

  职责：
  1. 全局导航条（AppNavbar，登录页除外）+ <router-view /> 路由出口
  2. 挂载时通过 getAppAuthStore() 初始化当前应用的认证状态（各应用已在 stores/auth.ts 注册 getter）
  3. 全局字体/布局样式由共享 @shared/style.css 提供，本组件不再内嵌 <style>

  不使用共享外壳的应用：
  - fj200c_main（AppNavbar #actions 插槽 + dashboard/theme/WS 常驻连接等角色专有逻辑）
  - city3d（暗黑主题变体，body/#app 样式不同）
-->
<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { AppNavbar, getAppAuthStore } from '@shared'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'

/** 路由实例，用于判断当前是否处于登录页 */
const route = useRoute()
/** 登录页不显示导航条，其余页面全局显示 */
const isLoginPage = computed(() => route.path.startsWith('/login'))

/** 当前应用的认证 Store 所需的最小接口（完整形态由各应用 stores/auth.ts 提供） */
interface AppAuthStore {
  initAuth: () => Promise<void>;
}

/** 当前应用的认证 Store 实例 */
const authStore = getAppAuthStore<AppAuthStore>()

/** 组件挂载后初始化认证状态（从 localStorage 恢复 token 并向后端校验） */
onMounted(() => {
  authStore?.initAuth()
})
</script>

<template>
  <el-config-provider :locale="zhCn">
    <div id="app">
      <!-- 全局导航条（登录页除外），所有角色所有页面共用 -->
      <AppNavbar v-if="!isLoginPage" />
      <router-view />
    </div>
  </el-config-provider>
</template>
