<!--
/**
 * @component AppNavbar
 * @description 通用导航栏组件
 *
 * 本组件是所有前端应用共享的导航栏，根据角色注册表动态生成菜单。
 *
 * ## 功能特性
 *
 * ### 动态菜单
 * - 根据用户角色从角色注册表（后端 /api/meta/roles 同步）获取权限，从 MENU_CONFIG 获取菜单
 * - 支持多级菜单（下拉菜单）
 * - 根据路由路径高亮当前菜单项
 *
 * ### 用户信息展示
 * - 显示用户名和角色
 * - 显示用户头像（首字母）
 * - 支持退出登录
 *
 * ### 响应式设计
 * - 桌面端：完整显示菜单和用户信息
 * - 平板端：菜单紧凑显示
 * - 移动端：隐藏菜单，只显示用户信息
 *
 * ## 架构设计
 *
 * ### 依赖注入
 * 本组件不直接依赖具体应用的 Store，而是通过 `getAppAuthStore()` 获取。
 * 这确保了：
 * 1. 组件可以在任何应用中使用
 * 2. 不需要传入 Store 实例
 * 3. 自动适配当前应用的 Store
 *
 * ### 插槽扩展
 * 提供 `actions` 插槽，允许应用在导航栏添加自定义按钮。
 * 例如：fj200c_information 应用可以添加"创建服务"按钮。
 *
 * ### 自动隐藏（autoHide）
 * 导航栏右侧提供「自动隐藏」复选框，勾选后启用：鼠标移出导航栏区域（超过导航栏高度）
 * 后 250ms 自动上滑隐藏；鼠标移到视口顶部（导航栏高度内）时重新滑出。
 * 默认不勾选（导航栏常驻）。也可通过 `autoHide` prop 控制初始勾选状态。
 * 隐藏通过外层 slot（普通元素）高度 64px→0 折叠实现，导航栏本体保持 64px 恒定；
 * 避免在 sticky 元素上做高度动画（部分运行环境下 sticky 元素高度重算失效、
 * 折叠后无法恢复显示）。
 *
 * ### 图标映射
 * 内置 Element Plus 图标映射，新角色菜单只需指定图标名称即可。
 *
 * ## 使用示例
 * ```vue
 * <template>
 *   <AppNavbar>
 *     <template #actions>
 *       <el-button type="primary" @click="createService">创建服务</el-button>
 *     </template>
 *   </AppNavbar>
 * </template>
 * ```
 -->
<template>
  <!--
    折叠容器（普通块元素，高度重算可靠）
    - 负责 64px ↔ 0 的折叠动画，隐藏后下方内容自动上移铺满视口
    - 不使用 sticky + 高度动画（sticky 元素的高度重算在部分运行环境失效）
  -->
  <header :class="{ 'navbar-hidden': !visible }" class="app-navbar-slot">
    <!-- 导航栏本体：恒 64px，不做高度动画 -->
    <nav class="app-navbar">
      <!-- 内容容器：左右布局 -->
      <div class="nav-content">
        <!-- 左侧：品牌和菜单 -->
        <div class="nav-left">
          <!-- 品牌区域：点击返回首页 -->
          <div class="nav-brand" @click="goHome">
            <el-icon class="brand-icon" size="24">
              <Monitor/>
            </el-icon>
            <!-- 品牌文字：显示当前角色名（注册表驱动） -->
            <span class="brand-text">{{ brandText }}</span>
          </div>

          <!-- 菜单区域：动态生成菜单项 -->
          <div class="nav-menu">
            <!--
              遍历菜单项
              - `v-for="menu in menuItems"`: 遍历菜单数组
              - `:key="menu.id"`: 为每个菜单项提供唯一 key（Vue 虚拟 DOM 优化需要）
            -->
            <template v-for="menu in menuItems" :key="menu.id">
              <!--
                无子菜单的项目
                - `v-if="!menu.children || menu.children.length === 0"`: 判断是否有子菜单
                - `<router-link>`: Vue Router 组件，用于声明式导航
                - `:to="menu.path"`: 目标路由路径
                - `:class="{ active: isActive(menu.path) }"`: 动态类绑定，当前路径匹配时添加 active 类
              -->
              <router-link
                  v-if="!menu.children || menu.children.length === 0"
                  :class="{ active: isActive(menu.path) }"
                  :to="menu.path"
                  class="nav-item"
              >
                <!--
                  动态图标组件
                  - `<component :is="getIcon(menu.icon)" />`: 动态渲染图标组件
                  - `getIcon()`: 根据图标名称返回对应的 Vue 组件
                -->
                <el-icon>
                  <component :is="getIcon(menu.icon)"/>
                </el-icon>
                <span>{{ menu.title }}</span>
              </router-link>

              <!--
                有子菜单的项目（下拉菜单）
                - `<el-dropdown>`: Element Plus 下拉菜单组件
                - `trigger="hover"`: 鼠标悬停触发
                - `@command="handleMenuCommand"`: 菜单项点击事件处理
              -->
              <el-dropdown
                  v-else
                  :key="menu.id"
                  trigger="hover"
                  @command="handleMenuCommand"
              >
                <!-- 下拉触发器 -->
                <span :class="{ active: isActive(menu.path) }" class="nav-item dropdown-item">
                <el-icon><component :is="getIcon(menu.icon)"/></el-icon>
                <span>{{ menu.title }}</span>
                <el-icon><ArrowDown/></el-icon>
              </span>
                <!-- 下拉菜单内容（具名插槽） -->
                <template #dropdown>
                  <el-dropdown-menu>
                    <!--
                      遍历子菜单
                      - `command="child.path"`: 点击时传递路径作为命令
                    -->
                    <el-dropdown-item
                        v-for="child in menu.children"
                        :key="child.id"
                        :command="child.path"
                    >
                      <el-icon>
                        <component :is="getIcon(child.icon)"/>
                      </el-icon>
                      {{ child.title }}
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </template>
          </div>
        </div>

        <!-- 右侧：自定义动作和用户信息 -->
        <div class="nav-right">
          <!--
            应用自定义动作区域（插槽）
            - 应用可以通过此插槽添加自定义按钮
            - 例如：创建服务、导出数据等
          -->
          <div class="nav-actions">
            <!-- 自动隐藏开关 -->
            <el-checkbox
                v-model="autoHideEnabled"
                class="auto-hide-toggle"
                size="small"
            >
              自动隐藏
            </el-checkbox>
            <slot name="actions"></slot>
          </div>

          <!-- 用户下拉菜单 -->
          <el-dropdown @command="handleCommand">
            <!-- 用户信息展示 -->
            <span class="user-dropdown">
            <!-- 用户头像（首字母） -->
            <el-avatar :size="32">
              {{ user?.username?.charAt(0)?.toUpperCase() }}
            </el-avatar>
              <!-- 用户名（桌面端显示） -->
            <span class="username desktop-only">{{ user?.username }}</span>
              <!-- 用户角色（桌面端显示） -->
            <span class="user-role desktop-only">({{ getUserRoleText(user?.role) }})</span>
            <el-icon><ArrowDown/></el-icon>
          </span>
            <!-- 下拉菜单内容 -->
            <template #dropdown>
              <el-dropdown-menu>
                <!-- 退出登录选项 -->
                <el-dropdown-item command="logout" divided>退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
    </nav>
  </header>
</template>

<script lang="ts" setup>
/**
 * 脚本部分
 *
 * 使用 Vue 3 Composition API（<script setup> 语法糖）
 *
 * 导入说明：
 * - `computed`: Vue 3 响应式 API，创建计算属性
 * - `useRouter`: Vue Router 组合式函数，获取路由实例
 * - `ElMessage`: Element Plus 消息提示组件
 * - Element Plus 图标：全量导入，确保所有图标可用
 * - 共享包：getAppAuthStore, findRole, 类型定义
 */

import {computed, onBeforeUnmount, onMounted, ref, watch} from 'vue'
import {useRouter} from 'vue-router'
import {ElMessage} from 'element-plus'
// 导入 Element Plus 图标组件
// 注意：这里全量导入所有图标，确保新角色菜单图标直接可用
import {
  ArrowDown,
  DataBoard,
  DataLine,
  Document,
  Files,
  Histogram,
  List,
  Lock,
  Monitor,
  OfficeBuilding,
  Plus,
  QuestionFilled,
  Setting,
  Tools,
  TrendCharts,
  User as UserIcon,
  UserFilled
} from '@element-plus/icons-vue'
// 从共享包导入工具函数和类型
import {findRole, getAppAuthStore, type MenuItem, type User} from '..'

// ============ 类型定义 ============

/**
 * Auth Store 接口形状
 *
 * 定义本组件需要的 Store 最小接口。
 * 这确保了：
 * 1. 组件不依赖具体 Store 实现
 * 2. 只依赖必要的属性和方法
 * 3. 支持 TypeScript 类型检查
 *
 * 注意：
 * - 这是组件内部类型，不对外导出
 * - 实际 Store 可能包含更多属性和方法
 */
interface AuthStoreShape {
  /** 用户信息（null 表示未登录） */
  user: User | null
  /** 菜单列表（根据角色和应用类型生成） */
  menuItems: MenuItem[]
  /** 登出方法 */
  logout: () => void
}

// ============ 组合式函数 ============

/**
 * 导航栏自动隐藏配置
 *
 * - `autoHide`: 初始是否启用自动隐藏（默认 false，由导航栏内复选框控制）
 */
const props = withDefaults(
    defineProps<{
      autoHide?: boolean
    }>(),
    {autoHide: false},
)

/**
 * 自动隐藏开关状态（复选框 v-model）
 *
 * 默认不勾选（导航栏常驻），勾选后启用鼠标移出隐藏。
 */
const autoHideEnabled = ref(props.autoHide)

/**
 * 自动隐藏状态
 *
 * - `visible`: 导航栏是否可见
 * - `hideTimer`: 隐藏延迟定时器（移出后短暂延迟，防止快速移动误隐藏）
 *
 * 初始为可见：加载完成后导航栏先正常展示，鼠标移出隐藏区后才自动上滑隐藏；
 * 这样登录/进入页面始终能看到导航栏，避免"默认隐藏且无法恢复"的观感。
 */
const visible = ref(true)
let hideTimer: number | undefined

/** 视口顶部滑出触发区（导航栏高度） */
const SHOW_ZONE = 64
/** 移出导航栏后隐藏触发区（导航栏高度 + 缓冲区） */
const HIDE_ZONE = 96

/**
 * 鼠标移动监听
 *
 * 规则：
 * - 鼠标位于导航栏或打开的下拉菜单上：保持/显示
 * - 鼠标在视口顶部触发区内：显示
 * - 鼠标移出隐藏触发区：延迟隐藏
 */
const onMouseMove = (e: MouseEvent) => {
  if (!autoHideEnabled.value) return
  const target = e.target as HTMLElement | null
  const overNav = !!target?.closest('.app-navbar, .el-dropdown-menu, .el-dropdown__popper')
  if (overNav || e.clientY <= SHOW_ZONE) {
    clearTimeout(hideTimer)
    visible.value = true
  } else if (e.clientY > HIDE_ZONE) {
    clearTimeout(hideTimer)
    hideTimer = window.setTimeout(() => {
      visible.value = false
    }, 250)
  }
}

/**
 * 同步自动隐藏监听器
 *
 * 勾选时注册 mousemove 监听，取消时移除；切换瞬间始终先恢复导航栏可见。
 */
const applyAutoHide = () => {
  visible.value = true
  clearTimeout(hideTimer)
  if (autoHideEnabled.value) {
    document.addEventListener('mousemove', onMouseMove, {passive: true})
  } else {
    document.removeEventListener('mousemove', onMouseMove)
  }
}

onMounted(applyAutoHide)

watch(autoHideEnabled, applyAutoHide)

onBeforeUnmount(() => {
  document.removeEventListener('mousemove', onMouseMove)
  clearTimeout(hideTimer)
})

/**
 * 路由实例
 *
 * 用于：
 * - 编程式导航（router.push）
 * - 获取当前路由信息（router.currentRoute）
 */
const router = useRouter()

/**
 * 认证 Store 实例
 *
 * 通过 `getAppAuthStore()` 获取当前应用的 Store。
 * 使用 TypeScript 泛型指定接口形状。
 *
 * 注意：
 * - 如果 Store 未注册，返回 undefined
 * - 组件会安全处理 undefined 情况
 */
const authStore = getAppAuthStore<AuthStoreShape>()

// ============ 计算属性 ============

/**
 * 用户信息
 *
 * 从 Store 中获取用户信息，未登录返回 null。
 *
 * 使用场景：
 * - 显示用户名和头像
 * - 检查登录状态
 */
const user = computed(() => authStore?.user ?? null)

/**
 * 品牌文字
 *
 * 显示当前登录角色的名称。
 * 未登录时显示 "RustWeb"。
 *
 * 逻辑：
 * 1. 获取用户角色
 * 2. 从角色注册表查找角色名称
 * 3. 如果未找到，直接显示角色 key
 * 4. 如果未登录，显示默认名称
 */
const brandText = computed(() => {
  const role = authStore?.user?.role
  return role ? (findRole(role)?.name ?? role) : 'RustWeb'
})

/**
 * 菜单项列表
 *
 * 从 Store 中获取菜单列表，未登录返回空数组。
 *
 * 使用场景：
 * - 动态渲染导航菜单
 * - 检查是否有可访问的菜单
 */
const menuItems = computed(() => authStore?.menuItems ?? [])

// ============ 方法 ============

/**
 * 返回首页
 *
 * 点击品牌区域时触发。
 * 跳转到当前角色菜单的第一个面板。
 *
 * 逻辑：
 * 1. 获取第一个菜单项
 * 2. 如果有子菜单，跳转到第一个子菜单
 * 3. 如果没有子菜单，跳转到菜单本身的路径
 * 4. 如果没有菜单，跳转到登录页
 */
const goHome = (): void => {
  const first = menuItems.value[0]?.children?.[0]?.path ?? menuItems.value[0]?.path
  router.push(first ?? '/login')
}

/**
 * 检查路径是否激活
 *
 * 判断给定路径是否与当前路由匹配。
 * 支持精确匹配和前缀匹配。
 *
 * @param {string} path - 要检查的路径
 * @returns {boolean} 是否激活
 *
 * 匹配规则：
 * - 精确匹配：`/users` 匹配 `/users`
 * - 前缀匹配：`/users` 匹配 `/users/123`
 *
 * 使用场景：
 * - 菜单项高亮
 * - 当前页面标识
 */
const isActive = (path: string): boolean => {
  return router.currentRoute.value.path === path ||
      router.currentRoute.value.path.startsWith(path + '/')
}

/**
 * 获取图标组件
 *
 * 根据图标名称返回对应的 Vue 组件。
 *
 * @param {string} iconName - 图标名称（Element Plus 图标名）
 * @returns {Component} 图标组件
 *
 * 图标映射：
 * - 内置所有 Element Plus 图标
 * - 如果图标不存在，返回 Monitor 作为默认
 *
 * 使用场景：
 * - 动态渲染菜单图标
 * - 支持新角色菜单图标
 *
 * 注意：
 * - 使用 `Record<string, any>` 类型，因为图标组件类型是 Component
 * - 如果需要更严格的类型，可以导入 Component 类型
 */
const getIcon = (iconName: string) => {
  // 图标映射表
  const iconMap: Record<string, any> = {
    Monitor,
    DataBoard,
    Document,
    UserIcon,
    Setting,
    List,
    Plus,
    UserFilled,
    Files,
    Lock,
    Tools,
    QuestionFilled,
    DataLine,
    TrendCharts,
    Histogram,
    OfficeBuilding,
  }
  // 返回对应图标，如果不存在返回默认图标
  return iconMap[iconName] || Monitor
}

/**
 * 获取用户角色文本
 *
 * 根据角色 key 从注册表获取显示名称。
 * 未知角色直接显示 key。
 *
 * @param {string} [role] - 角色 key（可选）
 * @returns {string} 角色显示名称
 *
 * 使用场景：
 * - 导航栏显示用户角色
 * - 用户信息提示
 */
const getUserRoleText = (role?: string): string => {
  return role ? (findRole(role)?.name ?? role) : ''
}

/**
 * 处理菜单命令
 *
 * 下拉菜单项点击时触发。
 *
 * @param {string} path - 目标路径
 *
 * 使用场景：
 * - 子菜单导航
 * - 动态菜单跳转
 */
const handleMenuCommand = (path: string): void => {
  router.push(path)
}

/**
 * 处理用户下拉菜单命令
 *
 * 用户下拉菜单项点击时触发。
 *
 * @param {string} command - 命令标识
 *
 * 支持的命令：
 * - `logout`: 退出登录
 *
 * 执行流程：
 * 1. 调用 Store 的 logout 方法
 * 2. 跳转到登录页
 * 3. 显示成功消息
 */
const handleCommand = (command: string): void => {
  switch (command) {
    case 'logout':
      // 调用 Store 的登出方法（清除会话）
      authStore?.logout()
      // 跳转到登录页
      router.push('/login')
      // 显示成功消息
      ElMessage.success('已退出登录')
      break
  }
}
</script>

<style scoped>
/*
 * 折叠容器（外层）：
 * - 普通块级元素（非 sticky），高度重算可靠
 * - 负责 64px ↔ 0 的折叠动画，隐藏后下方内容自动上移铺满视口
 */
.app-navbar-slot {
  position: relative;
  z-index: 100;
  height: 64px;
  overflow: hidden;
  flex: none;
  transition: height 0.3s ease;
  background: white;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

/* 自动隐藏：折叠为 0 */
.app-navbar-slot.navbar-hidden {
  height: 0;
  border-bottom-color: transparent;
}

/* 导航栏本体：恒 64px，不参与高度动画 */
.app-navbar {
  height: 64px;
}

.nav-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 64px;
  padding: 0 24px;
  max-width: 1920px;
  margin: 0 auto;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 32px;
}

.nav-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 20px;
  font-weight: 600;
  font-family: inherit;
  color: #409eff;
  cursor: pointer;
  transition: opacity 0.3s ease;
}

.nav-brand:hover {
  opacity: 0.8;
}

.brand-icon {
  color: #409eff;
}

.nav-menu {
  display: flex;
  gap: 8px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  text-decoration: none;
  color: #666;
  font-weight: 500;
  font-size: 14px;
  font-family: inherit;
  transition: all 0.3s ease;
  cursor: pointer;
  white-space: nowrap;
}

.nav-item:hover {
  background-color: #f0f9ff;
  color: #409eff;
}

.nav-item.active {
  background-color: #409eff;
  color: white;
}

.dropdown-item {
  position: relative;
}

.nav-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.auto-hide-toggle {
  margin-right: 4px;
  white-space: nowrap;
}

.user-dropdown {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 8px;
  border-radius: 6px;
  transition: background-color 0.3s ease;
  font-family: inherit;
}

.user-dropdown:hover {
  background-color: #f5f5f5;
}

.username {
  font-weight: 500;
  color: #333;
  font-size: 14px;
  font-family: inherit;
  white-space: nowrap;
}

.user-role {
  font-size: 12px;
  color: #999;
  font-family: inherit;
  white-space: nowrap;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .nav-content {
    padding: 0 16px;
  }

  .nav-left {
    gap: 16px;
  }

  .nav-menu {
    display: none;
  }

  .nav-item {
    font-size: 14px;
  }

  .username {
    font-size: 14px;
  }

  .user-role {
    font-size: 12px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .nav-menu {
    gap: 4px;
  }

  .nav-item {
    padding: 8px 12px;
  }
}
</style>
