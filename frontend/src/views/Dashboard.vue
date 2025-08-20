<template>
  <div class="dashboard-container">
    <!-- 头部导航 -->
    <header class="dashboard-header" :style="{ height: layoutConfig.header.height }">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">仪表板</h1>
        </div>
        <div class="header-right">
          <el-dropdown @command="handleCommand">
            <span class="user-dropdown">
              <el-avatar :size="32" :src="user?.avatar">
                {{ user?.username?.charAt(0)?.toUpperCase() }}
              </el-avatar>
              <span class="username desktop-only">{{ user?.username }}</span>
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="profile">个人资料</el-dropdown-item>
                <el-dropdown-item command="settings">设置</el-dropdown-item>
                <el-dropdown-item divided command="logout">退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
    </header>

    <!-- 主要内容区域 -->
    <main class="dashboard-main">
      <div class="container">
        <!-- 统计卡片 -->
        <div class="stats-grid">
          <el-card class="stat-card" shadow="hover">
            <div class="stat-content">
              <div class="stat-icon">
                <el-icon size="24"><Document /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-number">{{ stats.totalPosts }}</div>
                <div class="stat-label">总文章数</div>
              </div>
            </div>
          </el-card>

          <el-card class="stat-card" shadow="hover">
            <div class="stat-content">
              <div class="stat-icon">
                <el-icon size="24"><View /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-number">{{ stats.totalViews }}</div>
                <div class="stat-label">总浏览量</div>
              </div>
            </div>
          </el-card>

          <el-card class="stat-card" shadow="hover">
            <div class="stat-content">
              <div class="stat-icon">
                <el-icon size="24"><User /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-number">{{ stats.totalUsers }}</div>
                <div class="stat-label">用户数量</div>
              </div>
            </div>
          </el-card>

          <el-card class="stat-card" shadow="hover">
            <div class="stat-content">
              <div class="stat-icon">
                <el-icon size="24"><Star /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-number">{{ stats.avgRating }}</div>
                <div class="stat-label">平均评分</div>
              </div>
            </div>
          </el-card>
        </div>

        <!-- 最近文章 -->
        <div class="recent-posts-section">
          <div class="section-header">
            <h2 class="section-title">最近文章</h2>
            <el-button type="primary" @click="createPost" size="small">
              <el-icon><Plus /></el-icon>
              新建文章
            </el-button>
          </div>

          <el-card class="posts-card">
            <el-table
              :data="recentPosts"
              :size="layoutConfig.table.size"
              :stripe="layoutConfig.table.stripe"
              :border="layoutConfig.table.border"
              style="width: 100%"
            >
              <el-table-column prop="title" label="标题" min-width="200">
                <template #default="{ row }">
                  <router-link :to="`/posts/${row.id}`" class="post-link">
                    {{ row.title }}
                  </router-link>
                </template>
              </el-table-column>
              
              <el-table-column prop="status" label="状态" width="100">
                <template #default="{ row }">
                  <el-tag :type="getStatusType(row.status)" size="small">
                    {{ getStatusText(row.status) }}
                  </el-tag>
                </template>
              </el-table-column>
              
              <el-table-column prop="created_at" label="创建时间" width="180" class-name="desktop-only">
                <template #default="{ row }">
                  {{ formatDate(row.created_at) }}
                </template>
              </el-table-column>
              
              <el-table-column label="操作" width="150" fixed="right">
                <template #default="{ row }">
                  <el-button-group>
                    <el-button size="small" @click="editPost(row.id)">
                      <el-icon><Edit /></el-icon>
                    </el-button>
                    <el-button size="small" type="danger" @click="deletePost(row.id)">
                      <el-icon><Delete /></el-icon>
                    </el-button>
                  </el-button-group>
                </template>
              </el-table-column>
            </el-table>
          </el-card>
        </div>

        <!-- 快速操作 -->
        <div class="quick-actions-section">
          <h2 class="section-title">快速操作</h2>
          <div class="actions-grid">
            <el-card class="action-card" shadow="hover" @click="createPost">
              <div class="action-content">
                <el-icon size="32" class="action-icon"><Plus /></el-icon>
                <div class="action-text">新建文章</div>
              </div>
            </el-card>

            <el-card class="action-card" shadow="hover" @click="viewAllPosts">
              <div class="action-content">
                <el-icon size="32" class="action-icon"><Document /></el-icon>
                <div class="action-text">查看所有文章</div>
              </div>
            </el-card>

            <el-card class="action-card" shadow="hover" @click="viewProfile">
              <div class="action-content">
                <el-icon size="32" class="action-icon"><User /></el-icon>
                <div class="action-text">个人资料</div>
              </div>
            </el-card>

            <el-card class="action-card" shadow="hover" @click="viewSettings">
              <div class="action-content">
                <el-icon size="32" class="action-icon"><Setting /></el-icon>
                <div class="action-text">系统设置</div>
              </div>
            </el-card>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Document, View, User, Star, Plus, Edit, Delete,
  ArrowDown, Setting
} from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLayoutConfig } from '@/utils/responsive'
import type { Post } from '@/types'

const router = useRouter()
const authStore = useAuthStore()
const { layoutConfig } = useLayoutConfig()

const user = computed(() => authStore.user)

// 统计数据
const stats = ref({
  totalPosts: 0,
  totalViews: 0,
  totalUsers: 0,
  avgRating: 0
})

// 最近文章
const recentPosts = ref<Post[]>([])

// 获取状态类型
const getStatusType = (status: string) => {
  switch (status) {
    case 'published': return 'success'
    case 'draft': return 'info'
    case 'archived': return 'warning'
    default: return 'info'
  }
}

// 获取状态文本
const getStatusText = (status: string) => {
  switch (status) {
    case 'published': return '已发布'
    case 'draft': return '草稿'
    case 'archived': return '已归档'
    default: return status
  }
}

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleDateString('zh-CN')
}

// 处理下拉菜单命令
const handleCommand = (command: string) => {
  switch (command) {
    case 'profile':
      viewProfile()
      break
    case 'settings':
      viewSettings()
      break
    case 'logout':
      handleLogout()
      break
  }
}

// 创建文章
const createPost = () => {
  router.push('/posts/create')
}

// 编辑文章
const editPost = (id: string) => {
  router.push(`/posts/${id}/edit`)
}

// 删除文章
const deletePost = async (id: string) => {
  try {
    await ElMessageBox.confirm('确定要删除这篇文章吗？', '确认删除', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    // TODO: 调用删除API
    ElMessage.success('删除成功')
    loadRecentPosts()
  } catch {
    // 用户取消删除
  }
}

// 查看所有文章
const viewAllPosts = () => {
  router.push('/posts')
}

// 查看个人资料
const viewProfile = () => {
  router.push('/profile')
}

// 查看设置
const viewSettings = () => {
  router.push('/settings')
}

// 退出登录
const handleLogout = () => {
  authStore.logout()
  router.push('/login')
  ElMessage.success('已退出登录')
}

// 加载统计数据
const loadStats = async () => {
  // TODO: 调用API获取统计数据
  stats.value = {
    totalPosts: 12,
    totalViews: 1234,
    totalUsers: 56,
    avgRating: 4.5
  }
}

// 加载最近文章
const loadRecentPosts = async () => {
  // TODO: 调用API获取最近文章
  recentPosts.value = [
    {
      id: '1',
      title: '示例文章1',
      content: '这是示例文章的内容...',
      author_id: '1',
      status: 'published',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z'
    },
    {
      id: '2',
      title: '示例文章2',
      content: '这是示例文章的内容...',
      author_id: '1',
      status: 'draft',
      created_at: '2024-01-02T00:00:00Z',
      updated_at: '2024-01-02T00:00:00Z'
    }
  ]
}

onMounted(() => {
  loadStats()
  loadRecentPosts()
})
</script>

<style scoped>
.dashboard-container {
  min-height: 100vh;
  background-color: #f5f5f5;
}

/* 头部样式 */
.dashboard-header {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
  padding: 0 24px;
}

.header-title {
  font-size: 24px;
  font-weight: 600;
  color: #333;
  margin: 0;
}

.user-dropdown {
  display: flex;
  align-items: center;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 8px;
  transition: background-color 0.3s;
}

.user-dropdown:hover {
  background-color: #f5f5f5;
}

.username {
  margin: 0 8px;
  font-weight: 500;
}

/* 主要内容区域 */
.dashboard-main {
  padding: 24px 0;
}

/* 统计卡片 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
  margin-bottom: 32px;
}

.stat-card {
  border-radius: 12px;
  transition: transform 0.3s ease;
}

.stat-card:hover {
  transform: translateY(-4px);
}

.stat-content {
  display: flex;
  align-items: center;
  padding: 8px;
}

.stat-icon {
  width: 60px;
  height: 60px;
  border-radius: 12px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  margin-right: 16px;
}

.stat-info {
  flex: 1;
}

.stat-number {
  font-size: 28px;
  font-weight: 700;
  color: #333;
  line-height: 1;
}

.stat-label {
  font-size: 14px;
  color: #666;
  margin-top: 4px;
}

/* 最近文章区域 */
.recent-posts-section {
  margin-bottom: 32px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.section-title {
  font-size: 20px;
  font-weight: 600;
  color: #333;
  margin: 0;
}

.posts-card {
  border-radius: 12px;
}

.post-link {
  color: #409eff;
  text-decoration: none;
  font-weight: 500;
}

.post-link:hover {
  text-decoration: underline;
}

/* 快速操作区域 */
.quick-actions-section {
  margin-bottom: 32px;
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.action-card {
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.action-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
}

.action-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 24px 16px;
  text-align: center;
}

.action-icon {
  color: #667eea;
  margin-bottom: 12px;
}

.action-text {
  font-size: 16px;
  font-weight: 500;
  color: #333;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .header-content {
    padding: 0 16px;
  }
  
  .header-title {
    font-size: 20px;
  }
  
  .dashboard-main {
    padding: 16px 0;
  }
  
  .stats-grid {
    grid-template-columns: 1fr;
    gap: 16px;
    margin-bottom: 24px;
  }
  
  .stat-content {
    padding: 12px;
  }
  
  .stat-icon {
    width: 50px;
    height: 50px;
    margin-right: 12px;
  }
  
  .stat-number {
    font-size: 24px;
  }
  
  .section-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  
  .actions-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  
  .action-content {
    padding: 16px 12px;
  }
  
  .action-icon {
    font-size: 24px;
    margin-bottom: 8px;
  }
  
  .action-text {
    font-size: 14px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  
  .actions-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

/* 桌面端优化 */
@media (min-width: 1024px) {
  .stats-grid {
    grid-template-columns: repeat(4, 1fr);
  }
  
  .actions-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}
</style>
