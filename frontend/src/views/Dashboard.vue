<template>
  <div class="dashboard-container">
    <!-- 导航栏 -->
    <AppNavbar />

    <!-- 主要内容区域 -->
    <main class="dashboard-main">
      <div class="container">
        <!-- 欢迎区域 -->
        <div class="welcome-section">
          <div class="welcome-content">
            <h1 class="welcome-title">欢迎回来，{{ user?.username }}！</h1>
            <p class="welcome-subtitle">今天是 {{ currentDate }}，祝您工作愉快！</p>
          </div>
          <div class="welcome-actions">
            <el-button type="primary" @click="createPost" size="large">
              <el-icon><Plus /></el-icon>
              新建文章
            </el-button>
            <el-button @click="refreshData" size="large" :loading="refreshing">
              <el-icon><Refresh /></el-icon>
              刷新数据
            </el-button>
          </div>
        </div>

        <!-- 实时统计卡片 -->
        <div class="stats-grid">
          <el-card class="stat-card" shadow="hover">
            <div class="stat-content">
              <div class="stat-icon">
                <el-icon size="24"><Document /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-number">{{ stats.totalPosts }}</div>
                <div class="stat-label">总文章数</div>
                <div class="stat-trend" :class="stats.postsTrend > 0 ? 'positive' : 'negative'">
                  <el-icon><TrendCharts /></el-icon>
                  {{ Math.abs(stats.postsTrend) }}% {{ stats.postsTrend > 0 ? '增长' : '下降' }}
                </div>
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
                <div class="stat-trend" :class="stats.viewsTrend > 0 ? 'positive' : 'negative'">
                  <el-icon><TrendCharts /></el-icon>
                  {{ Math.abs(stats.viewsTrend) }}% {{ stats.viewsTrend > 0 ? '增长' : '下降' }}
                </div>
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
                <div class="stat-trend" :class="stats.usersTrend > 0 ? 'positive' : 'negative'">
                  <el-icon><TrendCharts /></el-icon>
                  {{ Math.abs(stats.usersTrend) }}% {{ stats.usersTrend > 0 ? '增长' : '下降' }}
                </div>
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
                <div class="stat-trend" :class="stats.ratingTrend > 0 ? 'positive' : 'negative'">
                  <el-icon><TrendCharts /></el-icon>
                  {{ Math.abs(stats.ratingTrend) }}% {{ stats.ratingTrend > 0 ? '增长' : '下降' }}
                </div>
              </div>
            </div>
          </el-card>
        </div>

        <!-- 实时数据图表 -->
        <div class="charts-section">
          <div class="chart-row">
            <el-card class="chart-card" shadow="hover">
              <template #header>
                <div class="chart-header">
                  <span>文章发布趋势</span>
                  <el-select v-model="chartPeriod" size="small" @change="updateChartData">
                    <el-option label="最近7天" value="7" />
                    <el-option label="最近30天" value="30" />
                    <el-option label="最近90天" value="90" />
                  </el-select>
                </div>
              </template>
              <div class="chart-content">
                <div class="chart-placeholder">
                  <el-icon size="48"><TrendCharts /></el-icon>
                  <p>图表区域 - 需要集成图表库</p>
                </div>
              </div>
            </el-card>

            <el-card class="chart-card" shadow="hover">
              <template #header>
                <span>用户活跃度</span>
              </template>
              <div class="chart-content">
                <div class="chart-placeholder">
                  <el-icon size="48"><DataLine /></el-icon>
                  <p>图表区域 - 需要集成图表库</p>
                </div>
              </div>
            </el-card>
          </div>
        </div>

        <!-- 最近文章 -->
        <div class="recent-posts-section">
          <div class="section-header">
            <h2 class="section-title">最近文章</h2>
            <el-button type="primary" @click="viewAllPosts" size="small">
              查看全部
              <el-icon><ArrowRight /></el-icon>
            </el-button>
          </div>

          <el-card class="posts-card">
            <el-table
              :data="recentPosts"
              :size="layoutConfig.table.size"
              :stripe="layoutConfig.table.stripe"
              :border="layoutConfig.table.border"
              style="width: 100%"
              v-loading="loading"
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
              
              <el-table-column prop="created_at" label="创建时间" width="180">
                <template #default="{ row }">
                  {{ formatDate(row.created_at) }}
                </template>
              </el-table-column>
              
              <el-table-column label="操作" width="150" fixed="right">
                <template #default="{ row }">
                  <el-button-group>
                    <el-button size="small" @click="viewPost(row.id)">
                      <el-icon><View /></el-icon>
                    </el-button>
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

        <!-- 系统状态 -->
        <div class="system-status-section">
          <el-card class="status-card" shadow="hover">
            <template #header>
              <span>系统状态</span>
            </template>
            <div class="status-grid">
              <div class="status-item">
                <div class="status-label">数据库连接</div>
                <div class="status-value">
                  <el-tag :type="systemStatus.database ? 'success' : 'danger'" size="small">
                    {{ systemStatus.database ? '正常' : '异常' }}
                  </el-tag>
                </div>
              </div>
              <div class="status-item">
                <div class="status-label">API 服务</div>
                <div class="status-value">
                  <el-tag :type="systemStatus.api ? 'success' : 'danger'" size="small">
                    {{ systemStatus.api ? '正常' : '异常' }}
                  </el-tag>
                </div>
              </div>
              <div class="status-item">
                <div class="status-label">系统负载</div>
                <div class="status-value">
                  <el-tag :type="systemStatus.load < 70 ? 'success' : systemStatus.load < 90 ? 'warning' : 'danger'" size="small">
                    {{ systemStatus.load }}%
                  </el-tag>
                </div>
              </div>
              <div class="status-item">
                <div class="status-label">内存使用</div>
                <div class="status-value">
                  <el-tag :type="systemStatus.memory < 70 ? 'success' : systemStatus.memory < 90 ? 'warning' : 'danger'" size="small">
                    {{ systemStatus.memory }}%
                  </el-tag>
                </div>
              </div>
            </div>
          </el-card>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Refresh, View, Edit, Delete, ArrowRight, TrendCharts, DataLine
} from '@element-plus/icons-vue'
import { useLayoutConfig } from '@/utils/responsive'
import { useAuthStore } from '@/stores/auth'
import { usePostsStore } from '@/stores/posts'
import { apiService } from '@/api'
import type { Post } from '@/types'
import AppNavbar from '@/components/AppNavbar.vue'

const router = useRouter()
const { layoutConfig } = useLayoutConfig()
const authStore = useAuthStore()
const postsStore = usePostsStore()

// 响应式数据
const loading = ref(false)
const refreshing = ref(false)
const chartPeriod = ref('7')

// 用户信息
const user = computed(() => authStore.user)

// 当前日期
const currentDate = computed(() => {
  return new Date().toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'long'
  })
})

// 统计数据
const stats = reactive({
  totalPosts: 0,
  totalViews: 0,
  totalUsers: 0,
  avgRating: 0,
  postsTrend: 0,
  viewsTrend: 0,
  usersTrend: 0,
  ratingTrend: 0
})

// 最近文章
const recentPosts = ref<Post[]>([])

// 系统状态
const systemStatus = reactive({
  database: true,
  api: true,
  load: 45,
  memory: 62
})

// 定时器
let refreshTimer: NodeJS.Timeout | null = null

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



// 创建文章
const createPost = () => {
  router.push('/posts/create')
}

// 刷新数据
const refreshData = async () => {
  refreshing.value = true
  try {
    await Promise.all([
      loadStats(),
      loadRecentPosts(),
      loadSystemStatus()
    ])
    ElMessage.success('数据刷新成功')
  } catch (error) {
    ElMessage.error('数据刷新失败')
  } finally {
    refreshing.value = false
  }
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
    
    const result = await postsStore.deletePost(id)
    if (result.success) {
      ElMessage.success('删除成功')
      await loadRecentPosts()
      await loadStats()
    } else {
      ElMessage.error(result.message || '删除失败')
    }
  } catch {
    // 用户取消删除
  }
}

// 查看文章
const viewPost = (id: string) => {
  router.push(`/posts/${id}`)
}

// 查看所有文章
const viewAllPosts = () => {
  router.push('/posts')
}




// 加载统计数据
const loadStats = async () => {
  try {
    // 获取文章统计
    const postsResponse = await apiService.getPosts()
    if (postsResponse.success && postsResponse.data) {
      const posts = postsResponse.data
      stats.totalPosts = posts.length
      stats.totalViews = posts.reduce((sum, post) => sum + (post.views || 0), 0)
      
      // 模拟趋势数据
      stats.postsTrend = Math.floor(Math.random() * 20) - 10
      stats.viewsTrend = Math.floor(Math.random() * 30) - 15
    }
    
    // 模拟用户和评分数据
    stats.totalUsers = Math.floor(Math.random() * 100) + 50
    stats.avgRating = (Math.random() * 2 + 3).toFixed(1)
    stats.usersTrend = Math.floor(Math.random() * 15) - 7
    stats.ratingTrend = Math.floor(Math.random() * 10) - 5
  } catch (error) {
    console.error('加载统计数据失败:', error)
  }
}

// 加载最近文章
const loadRecentPosts = async () => {
  try {
    const response = await apiService.getPosts()
    if (response.success && response.data) {
      // 按创建时间排序，取前5篇
      recentPosts.value = response.data
        .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
        .slice(0, 5)
    }
  } catch (error) {
    console.error('加载最近文章失败:', error)
  }
}

// 加载系统状态
const loadSystemStatus = async () => {
  try {
    // 模拟系统状态数据
    systemStatus.database = Math.random() > 0.1
    systemStatus.api = Math.random() > 0.05
    systemStatus.load = Math.floor(Math.random() * 80) + 20
    systemStatus.memory = Math.floor(Math.random() * 60) + 30
  } catch (error) {
    console.error('加载系统状态失败:', error)
  }
}

// 更新图表数据
const updateChartData = () => {
  console.log('更新图表数据，周期:', chartPeriod.value)
  // TODO: 根据选择的周期更新图表数据
}

// 自动刷新数据
const startAutoRefresh = () => {
  refreshTimer = setInterval(async () => {
    await loadStats()
    await loadSystemStatus()
  }, 30000) // 每30秒刷新一次
}

// 停止自动刷新
const stopAutoRefresh = () => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

onMounted(async () => {
  loading.value = true
  try {
    await Promise.all([
      loadStats(),
      loadRecentPosts(),
      loadSystemStatus()
    ])
  } catch (error) {
    console.error('初始化数据失败:', error)
  } finally {
    loading.value = false
  }
  
  // 启动自动刷新
  startAutoRefresh()
})

onUnmounted(() => {
  stopAutoRefresh()
})
</script>

<style scoped>
.dashboard-container {
  min-height: 100vh;
  background-color: #f5f5f5;
}



/* 主要内容 */
.dashboard-main {
  padding: 24px 0;
}

.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
}

/* 欢迎区域 */
.welcome-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 32px;
  padding: 24px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 12px;
  color: white;
}

.welcome-title {
  font-size: 28px;
  font-weight: 600;
  margin: 0 0 8px 0;
}

.welcome-subtitle {
  font-size: 16px;
  opacity: 0.9;
  margin: 0;
}

.welcome-actions {
  display: flex;
  gap: 12px;
}

/* 统计网格 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
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
  gap: 16px;
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
}

.stat-info {
  flex: 1;
}

.stat-number {
  font-size: 32px;
  font-weight: 700;
  color: #333;
  margin-bottom: 4px;
}

.stat-label {
  font-size: 14px;
  color: #666;
  margin-bottom: 8px;
}

.stat-trend {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 500;
}

.stat-trend.positive {
  color: #67c23a;
}

.stat-trend.negative {
  color: #f56c6c;
}

/* 图表区域 */
.charts-section {
  margin-bottom: 32px;
}

.chart-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
  gap: 24px;
}

.chart-card {
  border-radius: 12px;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-content {
  height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chart-placeholder {
  text-align: center;
  color: #999;
}

.chart-placeholder .el-icon {
  margin-bottom: 16px;
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

/* 系统状态 */
.system-status-section {
  margin-bottom: 32px;
}

.status-card {
  border-radius: 12px;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
}

.status-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid #f0f0f0;
}

.status-item:last-child {
  border-bottom: none;
}

.status-label {
  font-weight: 500;
  color: #333;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .welcome-section {
    flex-direction: column;
    gap: 16px;
    text-align: center;
  }
  
  .welcome-title {
    font-size: 24px;
  }
  
  .welcome-actions {
    flex-direction: column;
    width: 100%;
  }
  
  .stats-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .chart-row {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .status-grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  
  .chart-row {
    grid-template-columns: 1fr;
  }
}

/* 桌面端优化 */
@media (min-width: 1024px) {
  .stats-grid {
    grid-template-columns: repeat(4, 1fr);
  }
  
  .chart-row {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
