<template>
  <div class="profile-container">
    <!-- 导航栏 -->
    <AppNavbar />
    
    <!-- 页面头部 -->
    <header class="page-header">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">个人资料</h1>
        </div>
      </div>
    </header>

    <!-- 主要内容 -->
    <main class="profile-main">
      <div class="container">
        <div class="profile-grid">
          <!-- 个人信息卡片 -->
          <el-card class="profile-card">
            <template #header>
              <div class="card-header">
                <span>基本信息</span>
                <el-button 
                  type="primary" 
                  size="small" 
                  @click="editMode = !editMode"
                  :icon="editMode ? Close : Edit"
                >
                  {{ editMode ? '取消编辑' : '编辑资料' }}
                </el-button>
              </div>
            </template>

            <el-form
              ref="formRef"
              :model="form"
              :rules="rules"
              label-width="100px"
              :disabled="!editMode"
            >
              <el-form-item label="用户名" prop="username">
                <el-input v-model="form.username" placeholder="请输入用户名" />
              </el-form-item>

              <el-form-item label="邮箱" prop="email">
                <el-input v-model="form.email" placeholder="请输入邮箱" />
              </el-form-item>

              <el-form-item label="角色">
                <el-tag :type="getRoleType(form.role)" size="large">
                  {{ getRoleText(form.role) }}
                </el-tag>
              </el-form-item>

              <el-form-item label="注册时间">
                <span class="info-text">{{ formatDate(form.created_at) }}</span>
              </el-form-item>

              <el-form-item label="最后更新">
                <span class="info-text">{{ formatDate(form.updated_at) }}</span>
              </el-form-item>

              <el-form-item v-if="editMode">
                <el-button type="primary" @click="handleSubmit" :loading="loading">
                  保存修改
                </el-button>
                <el-button @click="resetForm">重置</el-button>
              </el-form-item>
            </el-form>
          </el-card>

          <!-- 修改密码卡片 -->
          <el-card class="password-card">
            <template #header>
              <span>修改密码</span>
            </template>

            <el-form
              ref="passwordFormRef"
              :model="passwordForm"
              :rules="passwordRules"
              label-width="100px"
            >
              <el-form-item label="当前密码" prop="currentPassword">
                <el-input
                  v-model="passwordForm.currentPassword"
                  type="password"
                  placeholder="请输入当前密码"
                  show-password
                />
              </el-form-item>

              <el-form-item label="新密码" prop="newPassword">
                <el-input
                  v-model="passwordForm.newPassword"
                  type="password"
                  placeholder="请输入新密码"
                  show-password
                />
              </el-form-item>

              <el-form-item label="确认密码" prop="confirmPassword">
                <el-input
                  v-model="passwordForm.confirmPassword"
                  type="password"
                  placeholder="请再次输入新密码"
                  show-password
                />
              </el-form-item>

              <el-form-item>
                <el-button 
                  type="primary" 
                  @click="handleChangePassword" 
                  :loading="passwordLoading"
                >
                  修改密码
                </el-button>
              </el-form-item>
            </el-form>
          </el-card>

          <!-- 账户统计卡片 -->
          <el-card class="stats-card">
            <template #header>
              <span>账户统计</span>
            </template>

            <div class="stats-grid">
              <div class="stat-item">
                <div class="stat-number">{{ stats.totalPosts }}</div>
                <div class="stat-label">文章总数</div>
              </div>
              <div class="stat-item">
                <div class="stat-number">{{ stats.publishedPosts }}</div>
                <div class="stat-label">已发布</div>
              </div>
              <div class="stat-item">
                <div class="stat-number">{{ stats.draftPosts }}</div>
                <div class="stat-label">草稿</div>
              </div>
              <div class="stat-item">
                <div class="stat-number">{{ stats.daysSinceJoin }}</div>
                <div class="stat-label">注册天数</div>
              </div>
            </div>
          </el-card>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Edit, Close, ArrowLeft } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { apiService } from '@/api'
import type { FormInstance, FormRules } from 'element-plus'
import AppNavbar from '@/components/AppNavbar.vue'

const authStore = useAuthStore()

// 响应式数据
const editMode = ref(false)
const loading = ref(false)
const passwordLoading = ref(false)
const formRef = ref<FormInstance>()
const passwordFormRef = ref<FormInstance>()

// 表单数据
const form = reactive({
  username: '',
  email: '',
  role: '',
  created_at: '',
  updated_at: ''
})

// 密码表单
const passwordForm = reactive({
  currentPassword: '',
  newPassword: '',
  confirmPassword: ''
})

// 统计数据
const stats = reactive({
  totalPosts: 0,
  publishedPosts: 0,
  draftPosts: 0,
  daysSinceJoin: 0
})

// 表单验证规则
const rules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 50, message: '用户名长度在 3 到 50 个字符', trigger: 'blur' }
  ],
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱格式', trigger: 'blur' }
  ]
}

// 密码验证规则
const passwordRules: FormRules = {
  currentPassword: [
    { required: true, message: '请输入当前密码', trigger: 'blur' }
  ],
  newPassword: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能少于 6 个字符', trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (rule, value, callback) => {
        if (value !== passwordForm.newPassword) {
          callback(new Error('两次输入的密码不一致'))
        } else {
          callback()
        }
      },
      trigger: 'blur'
    }
  ]
}

// 获取角色类型
const getRoleType = (role: string) => {
  switch (role) {
    case 'admin': return 'danger'
    case 'user': return 'primary'
    default: return 'info'
  }
}

// 获取角色文本
const getRoleText = (role: string) => {
  switch (role) {
    case 'admin': return '管理员'
    case 'user': return '普通用户'
    default: return role
  }
}

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

// 加载用户资料
const loadProfile = async () => {
  try {
    const response = await apiService.getProfile()
    if (response.success && response.data) {
      const user = response.data
      form.username = user.username
      form.email = user.email
      form.role = user.role
      form.created_at = user.created_at
      form.updated_at = user.updated_at
      
      // 计算注册天数
      const joinDate = new Date(user.created_at)
      const now = new Date()
      stats.daysSinceJoin = Math.floor((now.getTime() - joinDate.getTime()) / (1000 * 60 * 60 * 24))
    }
  } catch (error) {
    console.error('加载用户资料失败:', error)
    ElMessage.error('加载用户资料失败')
  }
}

// 加载统计数据
const loadStats = async () => {
  try {
    const response = await apiService.getPosts()
    if (response.success && response.data) {
      const posts = response.data
      stats.totalPosts = posts.length
      stats.publishedPosts = posts.filter(p => p.status === 'published').length
      stats.draftPosts = posts.filter(p => p.status === 'draft').length
    }
  } catch (error) {
    console.error('加载统计数据失败:', error)
  }
}

// 提交表单
const handleSubmit = async () => {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (valid) {
      loading.value = true
      try {
        const response = await apiService.updateProfile({
          username: form.username,
          email: form.email
        })
        
        if (response.success) {
          ElMessage.success('资料更新成功')
          editMode.value = false
          await loadProfile()
        } else {
          ElMessage.error(response.message || '更新失败')
        }
      } catch (error) {
        console.error('更新用户资料失败:', error)
        ElMessage.error('更新失败')
      } finally {
        loading.value = false
      }
    }
  })
}

// 重置表单
const resetForm = () => {
  if (formRef.value) {
    formRef.value.resetFields()
  }
  editMode.value = false
}

// 修改密码
const handleChangePassword = async () => {
  if (!passwordFormRef.value) return
  
  await passwordFormRef.value.validate(async (valid) => {
    if (valid) {
      passwordLoading.value = true
      try {
        const response = await apiService.changePassword({
          currentPassword: passwordForm.currentPassword,
          newPassword: passwordForm.newPassword
        })
        
        if (response.success) {
          ElMessage.success('密码修改成功')
          passwordForm.currentPassword = ''
          passwordForm.newPassword = ''
          passwordForm.confirmPassword = ''
          if (passwordFormRef.value) {
            passwordFormRef.value.resetFields()
          }
        } else {
          ElMessage.error(response.message || '密码修改失败')
        }
      } catch (error) {
        console.error('修改密码失败:', error)
        ElMessage.error('密码修改失败')
      } finally {
        passwordLoading.value = false
      }
    }
  })
}

onMounted(() => {
  loadProfile()
  loadStats()
})
</script>

<style scoped>
.profile-container {
  min-height: 100vh;
  background-color: #f5f5f5;
}

/* 页面头部样式 */
.page-header {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  position: sticky;
  top: 64px;
  z-index: 99;
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
.profile-main {
  padding: 24px 0;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
}

/* 网格布局 */
.profile-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.profile-card {
  grid-column: 1 / -1;
}

.password-card {
  grid-column: 1 / 2;
}

.stats-card {
  grid-column: 2 / 3;
}

/* 卡片头部 */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* 表单样式 */
.info-text {
  color: #666;
  font-size: 14px;
}

/* 统计网格 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 20px;
}

.stat-item {
  text-align: center;
  padding: 20px;
  background: #f8f9fa;
  border-radius: 8px;
}

.stat-number {
  font-size: 32px;
  font-weight: 700;
  color: #409eff;
  margin-bottom: 8px;
}

.stat-label {
  font-size: 14px;
  color: #666;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .header-content {
    padding: 0 16px;
  }
  
  .header-title {
    font-size: 20px;
  }
  
  .profile-main {
    padding: 16px 0;
  }
  
  .container {
    padding: 0 16px;
  }
  
  .profile-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .profile-card,
  .password-card,
  .stats-card {
    grid-column: 1 / -1;
  }
  
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  
  .stat-item {
    padding: 16px;
  }
  
  .stat-number {
    font-size: 24px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .profile-grid {
    grid-template-columns: 1fr;
    gap: 20px;
  }
  
  .password-card,
  .stats-card {
    grid-column: 1 / -1;
  }
}
</style>
