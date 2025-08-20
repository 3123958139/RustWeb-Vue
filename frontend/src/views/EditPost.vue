<template>
  <div class="edit-post-page">
    <el-container>
      <el-header class="header">
        <div class="header-content">
          <h1>编辑文章</h1>
          <el-button @click="$router.push(`/posts/${postId}`)">返回详情</el-button>
        </div>
      </el-header>
      
      <el-main class="main-content">
        <el-card v-loading="postsStore.loading">
          <template #header>
            <span>编辑文章</span>
          </template>
          
          <el-form
            v-if="postsStore.currentPost"
            ref="formRef"
            :model="form"
            :rules="rules"
            label-width="80px"
            @submit.prevent="handleSubmit"
          >
            <el-form-item label="标题" prop="title">
              <el-input
                v-model="form.title"
                placeholder="请输入文章标题"
                clearable
              />
            </el-form-item>
            
            <el-form-item label="内容" prop="content">
              <el-input
                v-model="form.content"
                type="textarea"
                :rows="10"
                placeholder="请输入文章内容"
                clearable
              />
            </el-form-item>
            
            <el-form-item label="状态" prop="status">
              <el-select v-model="form.status" placeholder="请选择状态">
                <el-option label="草稿" value="draft" />
                <el-option label="已发布" value="published" />
              </el-select>
            </el-form-item>
            
            <el-form-item>
              <el-button
                type="primary"
                :loading="loading"
                @click="handleSubmit"
              >
                保存修改
              </el-button>
              <el-button @click="$router.push(`/posts/${postId}`)">
                取消
              </el-button>
            </el-form-item>
          </el-form>
          
          <div v-else-if="!postsStore.loading" class="no-data">
            <el-empty description="文章不存在" />
          </div>
        </el-card>
      </el-main>
    </el-container>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { usePostsStore } from '@/stores/posts'
import type { FormInstance, FormRules } from 'element-plus'

const route = useRoute()
const router = useRouter()
const postsStore = usePostsStore()

const formRef = ref<FormInstance>()
const loading = ref(false)

const postId = computed(() => route.params.id as string)

const form = reactive({
  title: '',
  content: '',
  status: 'draft'
})

const rules: FormRules = {
  title: [
    { required: true, message: '请输入文章标题', trigger: 'blur' },
    { min: 1, max: 200, message: '标题长度在 1 到 200 个字符', trigger: 'blur' }
  ],
  content: [
    { required: true, message: '请输入文章内容', trigger: 'blur' }
  ],
  status: [
    { required: true, message: '请选择文章状态', trigger: 'change' }
  ]
}

// 监听当前文章数据变化，更新表单
watch(() => postsStore.currentPost, (post) => {
  if (post) {
    form.title = post.title
    form.content = post.content
    form.status = post.status
  }
}, { immediate: true })

const handleSubmit = async () => {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (valid) {
      loading.value = true
      try {
        const result = await postsStore.updatePost(postId.value, {
          title: form.title,
          content: form.content,
          status: form.status
        })
        
        if (result.success) {
          ElMessage.success('文章更新成功')
          router.push(`/posts/${postId.value}`)
        } else {
          ElMessage.error(result.message || '更新失败')
        }
      } catch (error) {
        ElMessage.error('更新失败')
      } finally {
        loading.value = false
      }
    }
  })
}

onMounted(() => {
  postsStore.fetchPost(postId.value)
})
</script>

<style scoped>
.edit-post-page {
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

.main-content {
  padding: 20px;
  background-color: #f5f5f5;
}

.no-data {
  padding: 40px 0;
}
</style>
