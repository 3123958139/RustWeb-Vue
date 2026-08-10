<!--
  管理后台 - 创建用户页

  功能：
  1. 管理员创建新用户并分配角色
  2. 表单验证（用户名长度、邮箱格式、密码一致性、角色必选）
  3. 创建成功后自动跳转回用户列表

  Composition API 用法：
  - ref() / reactive() 创建响应式数据
  - useRouter() 编程式导航
  - FormInstance 类型用于表单验证
-->
<template>
  <div class="create-user-container">
    <!-- 全局导航条由 App.vue 渲染（登录页除外） -->
    
    <header class="page-header">
      <div class="header-content">
        <h1 class="page-title">创建用户</h1>
        <el-button @click="$router.go(-1)">返回</el-button>
      </div>
    </header>

    <main class="page-main">
      <div class="content-wrapper">
        <el-form 
          ref="formRef" 
          :model="form" 
          :rules="rules" 
          label-width="100px"
          class="create-form"
        >
          <el-form-item label="用户名" prop="username">
            <el-input v-model="form.username" placeholder="请输入用户名" />
          </el-form-item>
          
          <el-form-item label="邮箱" prop="email">
            <el-input v-model="form.email" placeholder="请输入邮箱" />
          </el-form-item>
          
          <el-form-item label="密码" prop="password">
            <el-input 
              v-model="form.password" 
              type="password" 
              placeholder="请输入密码"
              show-password
            />
          </el-form-item>
          
          <el-form-item label="确认密码" prop="confirmPassword">
            <el-input 
              v-model="form.confirmPassword" 
              type="password" 
              placeholder="请再次输入密码"
              show-password
            />
          </el-form-item>
          
          <el-form-item label="角色" prop="role">
            <el-select v-model="form.role" placeholder="请选择角色">
              <el-option
                v-for="role in roles"
                :key="role.key"
                :label="role.name"
                :value="role.key"
              />
            </el-select>
          </el-form-item>
          
          <el-form-item>
            <el-button type="primary" @click="submitForm" :loading="loading">
              创建用户
            </el-button>
            <el-button @click="resetForm">重置</el-button>
          </el-form-item>
        </el-form>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { getAllRoles } from '@shared'
import { usersApi } from '@/api'

/** 路由实例 */
const router = useRouter()
/** 表单引用 */
const formRef = ref<FormInstance>()
/** 提交加载状态 */
const loading = ref(false)

/** 角色注册表（下拉框动态渲染，新增角色自动出现） */
const roles = getAllRoles()

/**
 * 创建用户表单数据
 * 默认角色为 'fj200c_information'（发动机监控模块）
 */
const form = reactive({
  username: '',
  email: '',
  password: '',
  confirmPassword: '',
  role: 'fj200c_information'
})

/**
 * 自定义验证器：确认密码
 *
 * Element Plus 自定义验证器格式：
 * @param _rule  - 验证规则对象（未使用，以下划线前缀标记）
 * @param value  - 当前字段的值
 * @param callback - 回调函数，传入 Error 对象表示验证失败，不传表示通过
 */
const validateConfirmPassword = (_rule: any, value: string, callback: any) => {
  if (value === '') {
    callback(new Error('请再次输入密码'))
  } else if (value !== form.password) {
    callback(new Error('两次输入密码不一致'))
  } else {
    callback() // 无参数调用表示验证通过
  }
}

/** 表单验证规则 */
const rules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '长度在 3 到 20 个字符', trigger: 'blur' }
  ],
  email: [
    { required: true, message: '请输入邮箱地址', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能少于6位', trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, validator: validateConfirmPassword, trigger: 'blur' }
  ],
  role: [
    { required: true, message: '请选择角色', trigger: 'change' }
  ]
}

/**
 * 提交表单
 *
 * 流程：
 * 1. 调用 formRef.value.validate() 触发表单验证
 * 2. 验证通过后调用 usersApi.createUser() 发送创建请求
 * 3. 创建成功后跳转回用户列表
 */
const submitForm = async () => {
  if (!formRef.value) return
  
  try {
    // validate() 返回 Promise，不传回调时使用 async/await
    await formRef.value.validate()
    loading.value = true

    const response = await usersApi.createUser({
      username: form.username,
      email: form.email,
      password: form.password,
      role: form.role,
    })
    if (!response.success) {
      ElMessage.error(response.message || '创建用户失败')
      return
    }

    ElMessage.success('用户创建成功')
    router.push('/users') // 跳转回用户列表
  } catch (error: any) {
    // error.response?.data?.message：可选链操作符，安全访问嵌套属性
    ElMessage.error(error.response?.data?.message || '创建用户失败')
  } finally {
    loading.value = false
  }
}

/**
 * 重置表单
 * resetFields() 是 Element Plus Form 组件的方法，
 * 会将所有字段重置为初始值并清除验证错误
 */
const resetForm = () => {
  if (!formRef.value) return
  formRef.value.resetFields()
}
</script>

<style scoped>
.create-user-container {
  min-height: 100vh;
  background-color: #f5f7fa;
}

.page-header {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  padding: 20px 0;
}

.header-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.page-title {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.page-main {
  padding: 24px;
}

.content-wrapper {
  max-width: 600px;
  margin: 0 auto;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  padding: 40px;
}

.create-form {
  width: 100%;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .page-main {
    padding: 16px;
  }
  
  .header-content {
    padding: 0 16px;
  }
  
  .content-wrapper {
    padding: 20px;
  }
}
</style>
