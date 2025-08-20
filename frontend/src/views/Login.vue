<template>
  <div class="login-container">
    <div class="login-wrapper">
      <el-card class="login-card" :body-style="{ padding: layoutConfig.card.padding }">
        <template #header>
          <div class="card-header">
            <h2 class="login-title">用户登录</h2>
            <p class="login-subtitle">欢迎回来</p>
          </div>
        </template>
        
        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          :label-width="layoutConfig.form.labelWidth"
          :label-position="layoutConfig.form.labelPosition"
          @submit.prevent="handleLogin"
          class="login-form"
        >
          <el-form-item label="邮箱" prop="email">
            <el-input
              v-model="form.email"
              type="email"
              placeholder="请输入邮箱"
              clearable
              size="large"
              :prefix-icon="Message"
            />
          </el-form-item>
          
          <el-form-item label="密码" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              placeholder="请输入密码"
              show-password
              clearable
              size="large"
              :prefix-icon="Lock"
              @keyup.enter="handleLogin"
            />
          </el-form-item>
          
          <el-form-item>
            <el-button
              type="primary"
              :loading="loading"
              @click="handleLogin"
              size="large"
              class="login-button"
            >
              <el-icon v-if="!loading"><Right /></el-icon>
              {{ loading ? '登录中...' : '立即登录' }}
            </el-button>
          </el-form-item>
          
          <div class="form-footer">
            <span class="footer-text">还没有账号？</span>
            <router-link to="/register" class="register-link">立即注册</router-link>
          </div>
        </el-form>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Message, Lock, Right } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLayoutConfig } from '@/utils/responsive'
import type { FormInstance, FormRules } from 'element-plus'

const router = useRouter()
const authStore = useAuthStore()
const { layoutConfig } = useLayoutConfig()

const formRef = ref<FormInstance>()
const loading = ref(false)

const form = reactive({
  email: '',
  password: ''
})

const rules: FormRules = {
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱格式', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能少于6位', trigger: 'blur' }
  ]
}

const handleLogin = async () => {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (valid) {
      loading.value = true
      try {
        const result = await authStore.login(form.email, form.password)
        if (result.success) {
          ElMessage.success('登录成功')
          router.push('/dashboard')
        } else {
          ElMessage.error(result.message || '登录失败')
        }
      } catch (error) {
        ElMessage.error('登录失败')
      } finally {
        loading.value = false
      }
    }
  })
}
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 16px;
}

.login-wrapper {
  width: 100%;
  max-width: 480px;
}

.login-card {
  width: 100%;
  border-radius: 16px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
  backdrop-filter: blur(10px);
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.card-header {
  text-align: center;
  padding: 24px 0 16px;
}

.login-title {
  color: #333;
  margin: 0 0 8px 0;
  font-size: 28px;
  font-weight: 600;
}

.login-subtitle {
  color: #666;
  margin: 0;
  font-size: 14px;
}

.login-form {
  margin-top: 16px;
}

.login-button {
  width: 100%;
  height: 48px;
  font-size: 16px;
  font-weight: 500;
  border-radius: 8px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border: none;
  transition: all 0.3s ease;
}

.login-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(102, 126, 234, 0.3);
}

.form-footer {
  text-align: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

.footer-text {
  color: #666;
  font-size: 14px;
}

.register-link {
  color: #667eea;
  text-decoration: none;
  margin-left: 8px;
  font-weight: 500;
  transition: color 0.3s ease;
}

.register-link:hover {
  color: #764ba2;
  text-decoration: underline;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .login-container {
    padding: 8px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  }
  
  .login-wrapper {
    max-width: 100%;
  }
  
  .login-card {
    border-radius: 12px;
    margin: 0;
  }
  
  .card-header {
    padding: 16px 0 12px;
  }
  
  .login-title {
    font-size: 24px;
  }
  
  .login-subtitle {
    font-size: 13px;
  }
  
  .login-form {
    margin-top: 12px;
  }
  
  .login-button {
    height: 44px;
    font-size: 15px;
  }
  
  .form-footer {
    margin-top: 20px;
    padding-top: 12px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .login-wrapper {
    max-width: 520px;
  }
  
  .login-card {
    border-radius: 14px;
  }
}

/* 桌面端优化 */
@media (min-width: 1024px) {
  .login-container {
    padding: 24px;
  }
  
  .login-wrapper {
    max-width: 560px;
  }
  
  .login-card {
    border-radius: 18px;
  }
  
  .login-title {
    font-size: 32px;
  }
  
  .login-subtitle {
    font-size: 16px;
  }
}

/* 动画效果 */
.login-card {
  animation: slideUp 0.6s ease-out;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 表单输入框优化 */
:deep(.el-input__wrapper) {
  border-radius: 8px;
  transition: all 0.3s ease;
}

:deep(.el-input__wrapper:hover) {
  box-shadow: 0 0 0 1px #667eea;
}

:deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.2);
}

/* 错误状态优化 */
:deep(.el-form-item.is-error .el-input__wrapper) {
  box-shadow: 0 0 0 1px #f56c6c;
}

/* 移动端表单标签优化 */
@media (max-width: 767px) {
  :deep(.el-form-item__label) {
    font-weight: 500;
    color: #333;
  }
  
  :deep(.el-input__wrapper) {
    border-radius: 6px;
  }
}
</style>
