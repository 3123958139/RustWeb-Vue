<template>
  <div class="posts-container">
    <!-- 头部 -->
    <header class="posts-header" :style="{ height: layoutConfig.header.height }">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">文章管理</h1>
        </div>
        <div class="header-right">
          <el-button type="primary" @click="createPost" size="large">
            <el-icon><Plus /></el-icon>
            新建文章
          </el-button>
        </div>
      </div>
    </header>

    <!-- 主要内容 -->
    <main class="posts-main">
      <div class="container">
        <!-- 搜索和筛选 -->
        <div class="filters-section">
          <el-card class="filters-card">
            <div class="filters-content">
              <div class="search-box">
                <el-input
                  v-model="searchQuery"
                  placeholder="搜索文章标题..."
                  clearable
                  size="large"
                  :prefix-icon="Search"
                  @input="handleSearch"
                />
              </div>
              
              <div class="filter-options">
                <el-select
                  v-model="statusFilter"
                  placeholder="状态筛选"
                  clearable
                  size="large"
                  @change="handleFilter"
                >
                  <el-option label="全部" value="" />
                  <el-option label="已发布" value="published" />
                  <el-option label="草稿" value="draft" />
                  <el-option label="已归档" value="archived" />
                </el-select>
                
                <el-select
                  v-model="sortBy"
                  placeholder="排序方式"
                  size="large"
                  @change="handleSort"
                >
                  <el-option label="最新创建" value="created_desc" />
                  <el-option label="最早创建" value="created_asc" />
                  <el-option label="最新更新" value="updated_desc" />
                  <el-option label="最早更新" value="updated_asc" />
                </el-select>
              </div>
            </div>
          </el-card>
        </div>

        <!-- 文章列表 -->
        <div class="posts-section">
          <div class="posts-header-row">
            <div class="posts-count">
              共 {{ filteredPosts.length }} 篇文章
            </div>
            <div class="view-toggle">
              <el-radio-group v-model="viewMode" size="large">
                <el-radio-button value="table">
                  <el-icon><Grid /></el-icon>
                </el-radio-button>
                <el-radio-button value="card">
                  <el-icon><List /></el-icon>
                </el-radio-button>
              </el-radio-group>
            </div>
          </div>

          <!-- 表格视图 -->
          <div v-if="viewMode === 'table'" class="table-view">
            <el-card class="table-card">
              <el-table
                :data="paginatedPosts"
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
                
                <el-table-column prop="created_at" label="创建时间" width="180" class-name="desktop-only">
                  <template #default="{ row }">
                    {{ formatDate(row.created_at) }}
                  </template>
                </el-table-column>
                
                <el-table-column prop="updated_at" label="更新时间" width="180" class-name="desktop-only">
                  <template #default="{ row }">
                    {{ formatDate(row.updated_at) }}
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

          <!-- 卡片视图 -->
          <div v-else class="card-view">
            <div class="posts-grid">
              <el-card
                v-for="post in paginatedPosts"
                :key="post.id"
                class="post-card"
                shadow="hover"
              >
                <div class="post-card-content">
                  <div class="post-header">
                    <h3 class="post-title">
                      <router-link :to="`/posts/${post.id}`" class="post-link">
                        {{ post.title }}
                      </router-link>
                    </h3>
                    <el-tag :type="getStatusType(post.status)" size="small">
                      {{ getStatusText(post.status) }}
                    </el-tag>
                  </div>
                  
                  <p class="post-excerpt">
                    {{ getExcerpt(post.content) }}
                  </p>
                  
                  <div class="post-meta">
                    <span class="post-date">
                      <el-icon><Calendar /></el-icon>
                      {{ formatDate(post.created_at) }}
                    </span>
                  </div>
                  
                  <div class="post-actions">
                    <el-button size="small" @click="viewPost(post.id)">
                      <el-icon><View /></el-icon>
                      查看
                    </el-button>
                    <el-button size="small" @click="editPost(post.id)">
                      <el-icon><Edit /></el-icon>
                      编辑
                    </el-button>
                    <el-button size="small" type="danger" @click="deletePost(post.id)">
                      <el-icon><Delete /></el-icon>
                      删除
                    </el-button>
                  </div>
                </div>
              </el-card>
            </div>
          </div>

          <!-- 分页 -->
          <div class="pagination-section">
            <el-pagination
              v-model:current-page="currentPage"
              v-model:page-size="pageSize"
              :page-sizes="[10, 20, 50, 100]"
              :total="filteredPosts.length"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="handleSizeChange"
              @current-change="handleCurrentChange"
            />
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Search, Grid, List, View, Edit, Delete, Calendar
} from '@element-plus/icons-vue'
import { useLayoutConfig } from '@/utils/responsive'
import type { Post } from '@/types'

const router = useRouter()
const { layoutConfig } = useLayoutConfig()

// 响应式数据
const loading = ref(false)
const posts = ref<Post[]>([])
const searchQuery = ref('')
const statusFilter = ref('')
const sortBy = ref('created_desc')
const viewMode = ref('table')
const currentPage = ref(1)
const pageSize = ref(20)

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

// 获取文章摘要
const getExcerpt = (content: string) => {
  return content.length > 100 ? content.substring(0, 100) + '...' : content
}

// 过滤后的文章
const filteredPosts = computed(() => {
  let filtered = posts.value

  // 搜索过滤
  if (searchQuery.value) {
    filtered = filtered.filter(post =>
      post.title.toLowerCase().includes(searchQuery.value.toLowerCase())
    )
  }

  // 状态过滤
  if (statusFilter.value) {
    filtered = filtered.filter(post => post.status === statusFilter.value)
  }

  // 排序
  filtered.sort((a, b) => {
    switch (sortBy.value) {
      case 'created_desc':
        return new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      case 'created_asc':
        return new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      case 'updated_desc':
        return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      case 'updated_asc':
        return new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime()
      default:
        return 0
    }
  })

  return filtered
})

// 分页后的文章
const paginatedPosts = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return filteredPosts.value.slice(start, end)
})

// 搜索处理
const handleSearch = () => {
  currentPage.value = 1
}

// 筛选处理
const handleFilter = () => {
  currentPage.value = 1
}

// 排序处理
const handleSort = () => {
  currentPage.value = 1
}

// 分页处理
const handleSizeChange = (size: number) => {
  pageSize.value = size
  currentPage.value = 1
}

const handleCurrentChange = (page: number) => {
  currentPage.value = page
}

// 创建文章
const createPost = () => {
  router.push('/posts/create')
}

// 查看文章
const viewPost = (id: string) => {
  router.push(`/posts/${id}`)
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
    loadPosts()
  } catch {
    // 用户取消删除
  }
}

// 加载文章列表
const loadPosts = async () => {
  loading.value = true
  try {
    // TODO: 调用API获取文章列表
    posts.value = [
      {
        id: '1',
        title: '示例文章1',
        content: '这是示例文章的内容，包含了很多有用的信息...',
        author_id: '1',
        status: 'published',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z'
      },
      {
        id: '2',
        title: '示例文章2',
        content: '这是另一篇示例文章的内容，同样包含了很多有用的信息...',
        author_id: '1',
        status: 'draft',
        created_at: '2024-01-02T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z'
      }
    ]
  } catch (error) {
    ElMessage.error('加载文章列表失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadPosts()
})
</script>

<style scoped>
.posts-container {
  min-height: 100vh;
  background-color: #f5f5f5;
}

/* 头部样式 */
.posts-header {
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

/* 主要内容 */
.posts-main {
  padding: 24px 0;
}

/* 筛选区域 */
.filters-section {
  margin-bottom: 24px;
}

.filters-card {
  border-radius: 12px;
}

.filters-content {
  display: flex;
  gap: 16px;
  align-items: center;
}

.search-box {
  flex: 1;
  max-width: 400px;
}

.filter-options {
  display: flex;
  gap: 12px;
}

/* 文章列表区域 */
.posts-section {
  margin-bottom: 24px;
}

.posts-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.posts-count {
  font-size: 16px;
  color: #666;
  font-weight: 500;
}

/* 表格视图 */
.table-card {
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

/* 卡片视图 */
.posts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
}

.post-card {
  border-radius: 12px;
  transition: transform 0.3s ease;
}

.post-card:hover {
  transform: translateY(-4px);
}

.post-card-content {
  padding: 16px;
}

.post-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.post-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  flex: 1;
  margin-right: 12px;
}

.post-title .post-link {
  color: #333;
}

.post-title .post-link:hover {
  color: #409eff;
}

.post-excerpt {
  color: #666;
  font-size: 14px;
  line-height: 1.5;
  margin-bottom: 12px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.post-meta {
  margin-bottom: 16px;
}

.post-date {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #999;
  font-size: 12px;
}

.post-actions {
  display: flex;
  gap: 8px;
}

/* 分页 */
.pagination-section {
  display: flex;
  justify-content: center;
  margin-top: 24px;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .header-content {
    padding: 0 16px;
  }
  
  .header-title {
    font-size: 20px;
  }
  
  .posts-main {
    padding: 16px 0;
  }
  
  .filters-content {
    flex-direction: column;
    gap: 12px;
  }
  
  .search-box {
    max-width: 100%;
  }
  
  .filter-options {
    width: 100%;
    justify-content: space-between;
  }
  
  .posts-header-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  
  .posts-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .post-card-content {
    padding: 12px;
  }
  
  .post-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  
  .post-title {
    margin-right: 0;
    font-size: 16px;
  }
  
  .post-actions {
    flex-wrap: wrap;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .posts-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

/* 桌面端优化 */
@media (min-width: 1024px) {
  .posts-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}

/* 大屏幕优化 */
@media (min-width: 1440px) {
  .posts-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}
</style>
