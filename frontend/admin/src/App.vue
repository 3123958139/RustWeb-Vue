<!--
  管理后台根组件

  作为整个应用的最外层容器，负责：
  1. 在组件挂载时初始化认证状态（从 localStorage 恢复 token 并校验）
  2. 通过 <router-view /> 渲染当前路由对应的页面组件
-->
<script setup lang="ts">
/**
 * <script setup> 是 Vue 3.2+ 的编译时语法糖：
 * - 所有顶层变量/函数自动暴露给模板，无需 return
 * - 等价于在 <script> 中使用 setup() 函数 + defineComponent
 * - 配合 lang="ts" 启用 TypeScript 类型检查
 */
import { onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { AppNavbar } from "@shared"
import zhCn from "element-plus/dist/locale/zh-cn.mjs"

/** 路由实例，用于判断当前是否处于登录页 */
const route = useRoute()
/** 登录页不显示导航条，其余页面全局显示 */
const isLoginPage = computed(() => route.path.startsWith('/login'))

/** 获取认证 Store 实例（Pinia 组合式写法） */
const authStore = useAuthStore()

/**
 * onMounted —— Vue 3 生命周期钩子
 * 在组件 DOM 挂载完成后执行，适合发起初始化请求
 */
onMounted(() => {
  // 从 localStorage 恢复 token，并向后端校验有效性
  authStore.initAuth()
})
</script>

<template>
  <!-- router-view 是 Vue Router 的内置组件，根据当前路由路径渲染对应页面 -->
  <el-config-provider :locale="zhCn">
  <div id="app">
    <!-- 全局导航条（登录页除外），所有角色所有页面共用 -->
    <AppNavbar v-if="!isLoginPage" />
    <router-view />
  </div>
  </el-config-provider>
</template>

<style>
/*
 * 全局样式（非 scoped），作用于整个应用
 * - scoped 样式只作用于当前组件，全局样式会穿透到所有子组件
 */
#app {
  font-family: 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', '微软雅黑', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;   /* WebKit 浏览器字体抗锯齿 */
  -moz-osx-font-smoothing: grayscale;   /* Firefox 字体抗锯齿 */
  height: 100vh;                         /* 铺满整个视口高度 */
}

/* CSS Reset：清除浏览器默认边距和内边距 */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;  /* 盒模型：width/height 包含 padding 和 border */
}

body {
  background-color: #f5f5f5;
}
</style>
