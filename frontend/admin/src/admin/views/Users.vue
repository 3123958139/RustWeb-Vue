<!--
  管理后台 - 用户列表页

  功能：
  1. 展示所有用户信息（用户名、邮箱、角色、注册时间）
  2. 支持按用户名/邮箱搜索和按角色筛选
  3. 支持编辑用户角色和删除用户
  4. 分页展示（Element Plus 分页组件）

  Composition API 用法：
  - ref() 创建响应式基本类型
  - computed() 创建计算属性（派生状态）
  - onMounted() 组件挂载后获取数据
  - useRouter() 编程式导航
-->
<template>
  <div class="users-container">
    <!-- 全局导航条由 App.vue 渲染（登录页除外） -->
    
    <header class="page-header">
      <div class="header-content">
        <h1 class="page-title">用户管理</h1>
        <div class="header-actions">
          <!-- 创建用户按钮：通过 hasPermission 检查权限，无权限时禁用 -->
          <el-button 
            type="primary" 
            @click="goCreateUser"
            :disabled="!authStore.hasPermission(Permission.UsersWrite)"
          >
            <el-icon><Plus /></el-icon>
            创建用户
          </el-button>
        </div>
      </div>
    </header>

    <main class="page-main">
      <div class="content-wrapper">
        <!-- 系统设置：初始密码查询停用开关 -->
        <div class="settings-section">
          <el-checkbox
            v-model="pwdRouteDisabled"
            @change="handlePwdRouteToggle"
          >
            停用初始密码查询（GET /admin/pwd）
          </el-checkbox>
          <span class="settings-hint">
            勾选后 /admin/pwd 不再返回种子账号初始密码（默认不勾选）
          </span>
        </div>

        <!-- 搜索和筛选区域 -->
        <div class="search-section">
          <el-input
            v-model="searchQuery"
            placeholder="搜索用户名或邮箱"
            class="search-input"
            clearable
            @input="handleSearch"
          >
            <template #prefix>
              <el-icon><Search /></el-icon>
            </template>
          </el-input>
          
          <el-select v-model="roleFilter" placeholder="角色筛选" clearable @change="handleSearch">
            <el-option label="全部角色" value="" />
            <el-option
              v-for="role in roles"
              :key="role.key"
              :label="role.name"
              :value="role.key"
            />
          </el-select>
        </div>

        <!-- 用户列表表格 -->
        <div class="users-table">
          <el-table 
            :data="filteredUsers" 
            v-loading="loading"
            stripe
            style="width: 100%"
          >
            <el-table-column prop="username" label="用户名" min-width="120">
              <template #default="{ row }">
                <div class="user-info">
                  <el-avatar :size="32" :src="row.avatar">
                    {{ row.username.charAt(0).toUpperCase() }}
                  </el-avatar>
                  <span class="username">{{ row.username }}</span>
                </div>
              </template>
            </el-table-column>
            
            <el-table-column prop="email" label="邮箱" min-width="200" />
            
            <el-table-column prop="role" label="角色" width="120">
              <template #default="{ row }">
                <el-tag :type="getRoleTagType(row.role)">
                  {{ getRoleText(row.role) }}
                </el-tag>
              </template>
            </el-table-column>
            
            <el-table-column prop="created_at" label="注册时间" width="180">
              <template #default="{ row }">
                {{ formatDate(row.created_at) }}
              </template>
            </el-table-column>
            
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button 
                  size="small" 
                  @click="editUserRole(row)"
                  :disabled="!authStore.hasPermission(Permission.UsersWrite)"
                >
                  编辑角色
                </el-button>
                <el-button 
                  size="small" 
                  type="danger" 
                  @click="deleteUser(row)"
                  :disabled="!authStore.hasPermission(Permission.UsersDelete)"
                >
                  删除
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <!-- 分页组件 -->
        <div class="pagination-wrapper">
          <el-pagination
            v-model:current-page="currentPage"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="totalUsers"
            layout="total, sizes, prev, pager, next, jumper"
            @size-change="handleSizeChange"
            @current-change="handleCurrentChange"
          />
        </div>
      </div>
    </main>

    <!-- 编辑角色对话框 -->
    <el-dialog 
      v-model="showRoleDialog" 
      title="编辑用户角色" 
      width="400px"
    >
      <el-form :model="editingUser" label-width="80px">
        <el-form-item label="用户名">
          <span>{{ editingUser.username }}</span>
        </el-form-item>
        <el-form-item label="当前角色">
          <el-tag :type="getRoleTagType(editingUser.role)">
            {{ getRoleText(editingUser.role) }}
          </el-tag>
        </el-form-item>
        <el-form-item label="新角色">
          <el-select v-model="newRole" placeholder="选择角色">
            <el-option
              v-for="role in roles"
              :key="role.key"
              :label="role.name"
              :value="role.key"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="showRoleDialog = false">取消</el-button>
          <el-button type="primary" @click="updateUserRole" :loading="updating">
            确认
          </el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus } from '@element-plus/icons-vue'
import { Permission, getAllRoles, findRole } from '@shared'
import type { User } from '@shared'
import { useAuthStore } from '@/stores/auth'
import { usersApi, settingsApi } from '@/api'

/** 认证 Store */
const authStore = useAuthStore()
/** 路由实例 */
const router = useRouter()

/** 角色注册表（动态获取所有角色，下拉框/筛选/标签自动适配新角色） */
const roles = getAllRoles()

// ========== 响应式数据 ==========

/** 表格加载状态 */
const loading = ref(false)
/** 用户列表数据 */
const users = ref<User[]>([])
/** 搜索关键词 */
const searchQuery = ref('')
/** 角色筛选值 */
const roleFilter = ref('')
/** 当前页码（v-model 双向绑定分页组件） */
const currentPage = ref(1)
/** 每页条数 */
const pageSize = ref(20)
/** 用户总数 */
const totalUsers = ref(0)

// ========== 系统设置：初始密码查询停用开关 ==========

/** 是否停用初始密码查询路由（GET /admin/pwd），默认不勾选 */
const pwdRouteDisabled = ref(false)

/** 获取停用状态（挂载时同步后端已保存的开关） */
const fetchPwdRouteStatus = async () => {
  try {
    const response = await settingsApi.getPwdRouteStatus()
    if (response.success && response.data) {
      pwdRouteDisabled.value = response.data.disabled
    }
  } catch {
    // 静默失败：保留默认值，不阻塞页面
  }
}

/**
 * 勾选/取消勾选切换停用状态
 * @param value - 勾选为 true（停用），取消为 false（启用）
 */
const handlePwdRouteToggle = async (value: boolean) => {
  try {
    const response = await settingsApi.setPwdRouteStatus(value)
    if (!response.success) {
      pwdRouteDisabled.value = !value // 失败时回滚勾选状态
      ElMessage.error(response.message || '设置失败')
      return
    }
    ElMessage.success(value ? '已停用初始密码查询' : '已启用初始密码查询')
  } catch {
    pwdRouteDisabled.value = !value // 失败时回滚勾选状态
    ElMessage.error('设置失败')
  }
}

// ========== 编辑角色相关 ==========

/** 是否显示编辑角色对话框（v-model 控制 el-dialog 显隐） */
const showRoleDialog = ref(false)
/** 当前正在编辑的用户 */
const editingUser = ref<User>({} as User)
/** 选择的新角色 */
const newRole = ref('')
/** 更新操作加载状态 */
const updating = ref(false)

/**
 * 计算属性：过滤后的用户列表
 *
 * computed() 会缓存结果，只有依赖的响应式数据（users/searchQuery/roleFilter）变化时才重新计算。
 * 这里实现了前端搜索和筛选，数据量大时应改为后端分页。
 */
const filteredUsers = computed(() => {
  let filtered = users.value

  // 搜索过滤：同时匹配用户名和邮箱（不区分大小写）
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    filtered = filtered.filter(user => 
      user.username.toLowerCase().includes(query) ||
      user.email.toLowerCase().includes(query)
    )
  }

  // 角色过滤
  if (roleFilter.value) {
    filtered = filtered.filter(user => user.role === roleFilter.value)
  }

  return filtered
})

/** 跳转到创建用户页面 */
const goCreateUser = () => {
  router.push('/users/create')
}

/**
 * 获取角色标签类型（Element Plus Tag 组件的 type 属性）
 * admin 角色显示红色（danger），其余显示灰色（info）
 */
const getRoleTagType = (role: string) => {
  return role === "admin" ? "danger" : "info"
}

/**
 * 获取角色显示文本
 * 通过 findRole() 从注册表查找角色名称，未知角色直接显示 key
 */
const getRoleText = (role: string) => {
  return findRole(role)?.name ?? role
}

/**
 * 格式化日期字符串为本地化格式
 * @param dateString - ISO 格式的日期字符串
 */
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

/** 搜索处理（重置到第一页，实际应调用后端搜索 API） */
const handleSearch = () => {
  currentPage.value = 1
}

/** 每页条数变化处理 */
const handleSizeChange = (size: number) => {
  pageSize.value = size
  currentPage.value = 1
  fetchUsers()
}

/** 页码变化处理 */
const handleCurrentChange = (page: number) => {
  currentPage.value = page
  fetchUsers()
}

/**
 * 获取用户列表
 *
 * async/await 语法：异步函数返回 Promise，await 等待 Promise 完成。
 * try/catch/finally 用于错误处理和资源清理。
 */
const fetchUsers = async () => {
  loading.value = true
  try {
    const response = await usersApi.getUsers()
    if (response.success && response.data) {
      users.value = response.data
      totalUsers.value = response.data.length
    } else {
      ElMessage.error(response.message || '获取用户列表失败')
    }
  } catch (error) {
    ElMessage.error('获取用户列表失败')
  } finally {
    loading.value = false // finally 块无论如何都会执行
  }
}

/**
 * 编辑用户角色
 * @param user - 要编辑的用户对象
 */
const editUserRole = (user: User) => {
  editingUser.value = { ...user } // 展开运算符浅拷贝，避免直接修改原数据
  newRole.value = user.role
  showRoleDialog.value = true
}

/**
 * 更新用户角色
 * 调用 API 后刷新列表，确保数据一致性
 */
const updateUserRole = async () => {
  if (!newRole.value) {
    ElMessage.warning('请选择新角色')
    return
  }

  updating.value = true
  try {
    const response = await usersApi.updateUserRole(editingUser.value.id, newRole.value)
    if (!response.success) {
      ElMessage.error(response.message || '角色更新失败')
      return
    }

    ElMessage.success('角色更新成功')
    showRoleDialog.value = false
    await fetchUsers() // 刷新列表
  } catch (error) {
    ElMessage.error('角色更新失败')
  } finally {
    updating.value = false
  }
}

/**
 * 删除用户
 *
 * 使用 ElMessageBox.confirm 弹出确认对话框，
 * 用户点击确定后执行删除操作。
 * ElMessageBox.confirm 返回 Promise，取消时 reject 'cancel'。
 */
const deleteUser = async (user: User) => {
  // 防止误删当前登录的管理员账号
  if (authStore.user?.id === user.id) {
    ElMessage.warning('不能删除当前登录的账号')
    return
  }

  try {
    await ElMessageBox.confirm(
      `确定要删除用户 "${user.username}" 吗？此操作不可恢复。`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )

    const response = await usersApi.deleteUser(user.id)
    if (!response.success) {
      ElMessage.error(response.message || '删除用户失败')
      return
    }

    // 前端同步移除已删除的用户（避免重新请求列表）
    users.value = users.value.filter(u => u.id !== user.id)
    totalUsers.value--

    ElMessage.success('用户删除成功')
  } catch (error) {
    // 用户点击取消时，ElMessageBox 会 reject 'cancel'，这不是真正的错误
    if (error !== 'cancel') {
      ElMessage.error('删除用户失败')
    }
  }
}

/** 组件挂载时获取用户列表 */
onMounted(() => {
  fetchUsers()
  fetchPwdRouteStatus()
})
</script>

<style scoped>
.users-container {
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

.header-actions {
  display: flex;
  gap: 12px;
}

.page-main {
  padding: 24px;
}

.content-wrapper {
  max-width: 1400px;
  margin: 0 auto;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.search-section {
  padding: 20px;
  border-bottom: 1px solid #e4e7ed;
  display: flex;
  gap: 16px;
  align-items: center;
}

.settings-section {
  padding: 16px 20px;
  border-bottom: 1px solid #e4e7ed;
  display: flex;
  align-items: center;
  gap: 12px;
  background: #fafbfd;
}

.settings-hint {
  font-size: 12px;
  color: #909399;
}

.search-input {
  width: 300px;
}

.users-table {
  padding: 0 20px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.username {
  font-weight: 500;
  color: #303133;
}

.pagination-wrapper {
  padding: 20px;
  display: flex;
  justify-content: center;
  border-top: 1px solid #e4e7ed;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .page-main {
    padding: 16px;
  }
  
  .header-content {
    padding: 0 16px;
    flex-direction: column;
    gap: 16px;
    align-items: flex-start;
  }
  
  .search-section {
    flex-direction: column;
    align-items: stretch;
  }
  
  .search-input {
    width: 100%;
  }
}
</style>
