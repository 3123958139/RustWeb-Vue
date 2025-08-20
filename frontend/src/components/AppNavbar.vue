<template>
  <nav class="app-navbar">
    <div class="nav-content">
      <div class="nav-left">
        <div class="nav-brand" @click="goHome">
          <el-icon size="24" class="brand-icon"><Monitor /></el-icon>
          <span class="brand-text">RustWeb</span>
        </div>
        <div class="nav-menu">
          <!-- 动态菜单项 -->
          <template v-for="menu in menuItems" :key="menu.id">
            <!-- 没有子菜单的项目 -->
            <router-link 
              v-if="!menu.children || menu.children.length === 0"
              :to="menu.path" 
              class="nav-item" 
              :class="{ active: isActive(menu.path) }"
            >
              <el-icon><component :is="getIcon(menu.icon)" /></el-icon>
              <span>{{ menu.title }}</span>
            </router-link>
            
            <!-- 有子菜单的项目 -->
            <el-dropdown 
              v-else 
              :key="menu.id"
              @command="handleMenuCommand"
              trigger="hover"
            >
              <span class="nav-item dropdown-item" :class="{ active: isActive(menu.path) }">
                <el-icon><component :is="getIcon(menu.icon)" /></el-icon>
                <span>{{ menu.title }}</span>
                <el-icon><ArrowDown /></el-icon>
              </span>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item 
                    v-for="child in menu.children" 
                    :key="child.id"
                    :command="child.path"
                  >
                    <el-icon><component :is="getIcon(child.icon)" /></el-icon>
                    {{ child.title }}
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </div>
      </div>
      <div class="nav-right">
        <el-dropdown @command="handleCommand">
          <span class="user-dropdown">
            <el-avatar :size="32" :src="user?.avatar">
              {{ user?.username?.charAt(0)?.toUpperCase() }}
            </el-avatar>
            <span class="username desktop-only">{{ user?.username }}</span>
            <span class="user-role desktop-only">({{ getUserRoleText(user?.role) }})</span>
            <el-icon><ArrowDown /></el-icon>
          </span>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="profile">个人资料</el-dropdown-item>
              <el-dropdown-item command="settings">设置</el-dropdown-item>
              <el-dropdown-item 
                v-if="authStore.hasPermission('SystemAdmin')" 
                command="menu-management"
              >
                菜单管理
              </el-dropdown-item>
              <el-dropdown-item 
                v-if="authStore.hasPermission('SystemAdmin')" 
                command="permission-management"
              >
                权限管理
              </el-dropdown-item>
              <el-dropdown-item divided command="logout">退出登录</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Monitor, DataBoard, Document, User, Setting, ArrowDown, List, Plus, UserFilled
} from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { UserRole } from '@/types'

const router = useRouter()
const authStore = useAuthStore()

// 用户信息
const user = computed(() => authStore.user)

// 菜单项
const menuItems = computed(() => authStore.menuItems)

// 返回首页
const goHome = () => {
  router.push('/dashboard')
}

// 检查路径是否激活
const isActive = (path: string) => {
  return router.currentRoute.value.path === path || 
         router.currentRoute.value.path.startsWith(path + '/')
}

// 获取图标组件
const getIcon = (iconName: string) => {
  const iconMap: Record<string, any> = {
    Monitor,
    DataBoard,
    Document,
    User,
    Setting,
    List,
    Plus,
    UserFilled,
  }
  return iconMap[iconName] || Monitor
}

// 获取用户角色文本
const getUserRoleText = (role?: string) => {
  switch (role) {
    case UserRole.Admin:
      return '管理员'
    case UserRole.Moderator:
      return '版主'
    case UserRole.User:
      return '用户'
    default:
      return '用户'
  }
}

// 处理菜单命令
const handleMenuCommand = (path: string) => {
  router.push(path)
}

// 处理下拉菜单命令
const handleCommand = (command: string) => {
  switch (command) {
    case 'profile':
      router.push('/profile')
      break
    case 'settings':
      router.push('/settings')
      break
    case 'menu-management':
      router.push('/menu-management')
      break
    case 'permission-management':
      router.push('/permission-management')
      break
    case 'logout':
      authStore.logout()
      router.push('/login')
      ElMessage.success('已退出登录')
      break
  }
}
</script>

<style scoped>
.app-navbar {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  position: sticky;
  top: 0;
  z-index: 100;
}

.nav-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 64px;
  padding: 0 24px;
  max-width: 1400px;
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
