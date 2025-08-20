<template>
  <div class="post-detail-page">
    <el-container>
      <el-header class="header">
        <div class="header-content">
          <h1>文章详情</h1>
          <div class="header-actions">
            <el-button @click="$router.push(`/posts/${postId}/edit`)">
              编辑
            </el-button>
            <el-button @click="$router.push('/posts')">返回列表</el-button>
          </div>
        </div>
      </el-header>
      
      <el-main class="main-content">
        <el-card v-loading="postsStore.loading">
          <template #header>
            <div class="card-header">
              <span>文章信息</span>
              <el-tag :type="postsStore.currentPost?.status === 'published' ? 'success' : 'warning'">
                {{ postsStore.currentPost?.status === 'published' ? '已发布' : '草稿' }}
              </el-tag>
            </div>
          </template>
          
          <div v-if="postsStore.currentPost" class="post-content">
            <h2 class="post-title">{{ postsStore.currentPost.title }}</h2>
            
            <div class="post-meta">
              <p><strong>创建时间：</strong>{{ formatDate(postsStore.currentPost.created_at) }}</p>
              <p><strong>更新时间：</strong>{{ formatDate(postsStore.currentPost.updated_at) }}</p>
              <p><strong>状态：</strong>
                <el-tag :type="postsStore.currentPost.status === 'published' ? 'success' : 'warning'">
                  {{ postsStore.currentPost.status === 'published' ? '已发布' : '草稿' }}
                </el-tag>
              </p>
            </div>
            
            <div class="post-body">
              <h3>文章内容：</h3>
              <div class="content-text">{{ postsStore.currentPost.content }}</div>
            </div>
          </div>
          
          <div v-else-if="!postsStore.loading" class="no-data">
            <el-empty description="文章不存在" />
          </div>
        </el-card>
      </el-main>
    </el-container>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { usePostsStore } from '@/stores/posts'

const route = useRoute()
const postsStore = usePostsStore()

const postId = computed(() => route.params.id as string)

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

onMounted(() => {
  postsStore.fetchPost(postId.value)
})
</script>

<style scoped>
.post-detail-page {
  height: 100vh;
}

.header {
  background-color: #fff;
  border-bottom: 1px solid #e4e7ed;
  padding: 0 20px;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
}

.header-content h1 {
  margin: 0;
  color: #303133;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.main-content {
  padding: 20px;
  background-color: #f5f5f5;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.post-content {
  padding: 20px 0;
}

.post-title {
  font-size: 2rem;
  color: #303133;
  margin-bottom: 20px;
  border-bottom: 2px solid #409eff;
  padding-bottom: 10px;
}

.post-meta {
  background-color: #f8f9fa;
  padding: 15px;
  border-radius: 4px;
  margin-bottom: 20px;
}

.post-meta p {
  margin: 5px 0;
  color: #606266;
}

.post-body h3 {
  color: #303133;
  margin-bottom: 15px;
}

.content-text {
  line-height: 1.8;
  color: #303133;
  white-space: pre-wrap;
  background-color: #fff;
  padding: 20px;
  border-radius: 4px;
  border: 1px solid #e4e7ed;
}

.no-data {
  padding: 40px 0;
}
</style>
