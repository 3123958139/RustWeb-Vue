<template>
  <div class="permission-management-container">
    <!-- 导航栏 -->
    <AppNavbar />
    
    <!-- 页面头部 -->
    <header class="page-header">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">权限管理</h1>
          <p class="header-subtitle">管理系统角色和权限配置</p>
        </div>
        <div class="header-right">
          <el-button type="primary" @click="showCreateRoleDialog = true" size="large">
            <el-icon><Plus /></el-icon>
            新建角色
          </el-button>
        </div>
      </div>
    </header>

    <!-- 主要内容 -->
    <main class="permission-main">
      <div class="container">
        <!-- 角色管理 -->
        <el-card class="role-card">
          <template #header>
            <div class="card-header">
              <span>角色管理</span>
              <div class="header-actions">
                <el-button @click="refreshRoles" size="small">
                  <el-icon><Refresh /></el-icon>
                  刷新
                </el-button>
              </div>
            </div>
          </template>

          <el-table
            :data="roles"
            :size="layoutConfig.table.size"
            :stripe="layoutConfig.table.stripe"
            :border="layoutConfig.table.border"
            v-loading="loading"
          >
            <el-table-column prop="name" label="角色名称" width="150">
              <template #default="{ row }">
                <div class="role-info">
                  <el-tag :type="getRoleType(row.name)" size="small">
                    {{ getRoleText(row.name) }}
                  </el-tag>
                </div>
              </template>
            </el-table-column>
            
            <el-table-column prop="description" label="描述" min-width="200">
              <template #default="{ row }">
                <span>{{ row.description }}</span>
              </template>
            </el-table-column>
            
            <el-table-column prop="permissions" label="权限" min-width="300">
              <template #default="{ row }">
                <div class="permissions-tags">
                  <el-tag 
                    v-for="perm in row.permissions" 
                    :key="perm" 
                    size="small" 
                    type="success"
                    class="permission-tag"
                  >
                    {{ getPermissionText(perm) }}
                  </el-tag>
                </div>
              </template>
            </el-table-column>
            
            <el-table-column prop="user_count" label="用户数" width="100" align="center">
              <template #default="{ row }">
                <span>{{ row.user_count || 0 }}</span>
              </template>
            </el-table-column>
            
            <el-table-column prop="created_at" label="创建时间" width="180">
              <template #default="{ row }">
                <span>{{ formatDate(row.created_at) }}</span>
              </template>
            </el-table-column>
            
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button-group>
                  <el-button size="small" @click="editRole(row)">
                    <el-icon><Edit /></el-icon>
                  </el-button>
                  <el-button size="small" @click="assignUsers(row)">
                    <el-icon><User /></el-icon>
                  </el-button>
                  <el-button 
                    size="small" 
                    type="danger" 
                    @click="deleteRole(row)"
                    :disabled="row.name === 'admin'"
                  >
                    <el-icon><Delete /></el-icon>
                  </el-button>
                </el-button-group>
              </template>
            </el-table-column>
          </el-table>
        </el-card>

        <!-- 用户角色分配 -->
        <el-card class="user-role-card">
          <template #header>
            <span>用户角色分配</span>
          </template>

          <div class="user-role-content">
            <div class="search-section">
              <el-input
                v-model="userSearchQuery"
                placeholder="搜索用户..."
                clearable
                size="large"
                :prefix-icon="Search"
                @input="handleUserSearch"
              />
            </div>

            <el-table
              :data="filteredUsers"
              :size="layoutConfig.table.size"
              :stripe="layoutConfig.table.stripe"
              :border="layoutConfig.table.border"
              v-loading="usersLoading"
            >
              <el-table-column prop="username" label="用户名" width="150">
                <template #default="{ row }">
                  <div class="user-info">
                    <el-avatar :size="32" :src="row.avatar">
                      {{ row.username?.charAt(0)?.toUpperCase() }}
                    </el-avatar>
                    <span class="username">{{ row.username }}</span>
                  </div>
                </template>
              </el-table-column>
              
              <el-table-column prop="email" label="邮箱" min-width="200">
                <template #default="{ row }">
                  <span>{{ row.email }}</span>
                </template>
              </el-table-column>
              
              <el-table-column prop="role" label="当前角色" width="150">
                <template #default="{ row }">
                  <el-tag :type="getRoleType(row.role)" size="small">
                    {{ getRoleText(row.role) }}
                  </el-tag>
                </template>
              </el-table-column>
              
              <el-table-column prop="created_at" label="注册时间" width="180">
                <template #default="{ row }">
                  <span>{{ formatDate(row.created_at) }}</span>
                </template>
              </el-table-column>
              
              <el-table-column label="操作" width="150" fixed="right">
                <template #default="{ row }">
                  <el-button size="small" @click="changeUserRole(row)">
                    <el-icon><Setting /></el-icon>
                    修改角色
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </el-card>
      </div>
    </main>

    <!-- 创建/编辑角色对话框 -->
    <el-dialog
      v-model="showCreateRoleDialog"
      :title="editingRole ? '编辑角色' : '新建角色'"
      width="700px"
      :close-on-click-modal="false"
    >
      <el-form
        ref="roleFormRef"
        :model="roleForm"
        :rules="roleFormRules"
        label-width="100px"
      >
        <el-form-item label="角色名称" prop="name">
          <el-input 
            v-model="roleForm.name" 
            placeholder="请输入角色名称"
            :disabled="editingRole?.name === 'admin'"
          />
        </el-form-item>
        
        <el-form-item label="角色描述" prop="description">
          <el-input 
            v-model="roleForm.description" 
            type="textarea"
            :rows="3"
            placeholder="请输入角色描述"
          />
        </el-form-item>
        
        <el-form-item label="权限配置">
          <div class="permissions-section">
            <div class="permission-group" v-for="group in permissionGroups" :key="group.name">
              <h4 class="group-title">{{ group.title }}</h4>
              <div class="permission-items">
                <el-checkbox
                  v-for="perm in group.permissions"
                  :key="perm.value"
                  v-model="roleForm.permissions"
                  :label="perm.value"
                  :disabled="perm.value === Permission.SystemAdmin && roleForm.name !== 'admin'"
                >
                  {{ perm.label }}
                </el-checkbox>
              </div>
            </div>
          </div>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="showCreateRoleDialog = false">取消</el-button>
          <el-button type="primary" @click="saveRole" :loading="saving">
            {{ editingRole ? '保存' : '创建' }}
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- 用户角色修改对话框 -->
    <el-dialog
      v-model="showUserRoleDialog"
      title="修改用户角色"
      width="500px"
      :close-on-click-modal="false"
    >
      <div class="user-role-dialog">
        <div class="user-info">
          <el-avatar :size="48" :src="selectedUser?.avatar">
            {{ selectedUser?.username?.charAt(0)?.toUpperCase() }}
          </el-avatar>
          <div class="user-details">
            <h4>{{ selectedUser?.username }}</h4>
            <p>{{ selectedUser?.email }}</p>
          </div>
        </div>
        
        <el-form label-width="100px">
          <el-form-item label="选择角色">
            <el-select v-model="selectedUserRole" placeholder="请选择角色">
              <el-option
                v-for="role in availableRoles"
                :key="role.name"
                :label="getRoleText(role.name)"
                :value="role.name"
              >
                <div class="role-option">
                  <el-tag :type="getRoleType(role.name)" size="small">
                    {{ getRoleText(role.name) }}
                  </el-tag>
                  <span class="role-desc">{{ role.description }}</span>
                </div>
              </el-option>
            </el-select>
          </el-form-item>
        </el-form>
      </div>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="showUserRoleDialog = false">取消</el-button>
          <el-button type="primary" @click="updateUserRole" :loading="updatingRole">
            确认修改
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Edit, Delete, Refresh, User, Setting, Search
} from '@element-plus/icons-vue'
import { useLayoutConfig } from '@/utils/responsive'
import { Permission, UserRole } from '@/types'
import AppNavbar from '@/components/AppNavbar.vue'

const { layoutConfig } = useLayoutConfig()

// 响应式数据
const loading = ref(false)
const usersLoading = ref(false)
const saving = ref(false)
const updatingRole = ref(false)
const showCreateRoleDialog = ref(false)
const showUserRoleDialog = ref(false)
const editingRole = ref<any>(null)
const selectedUser = ref<any>(null)
const selectedUserRole = ref('')
const userSearchQuery = ref('')

// 角色表单
const roleForm = reactive({
  id: '',
  name: '',
  description: '',
  permissions: [] as string[]
})

// 角色表单验证规则
const roleFormRules = {
  name: [
    { required: true, message: '请输入角色名称', trigger: 'blur' }
  ],
  description: [
    { required: true, message: '请输入角色描述', trigger: 'blur' }
  ]
}

// 权限分组
const permissionGroups = [
  {
    name: 'dashboard',
    title: '仪表盘',
    permissions: [
      { value: Permission.Dashboard, label: '仪表盘访问' }
    ]
  },
  {
    name: 'posts',
    title: '文章管理',
    permissions: [
      { value: Permission.PostsRead, label: '文章读取' },
      { value: Permission.PostsWrite, label: '文章写入' },
      { value: Permission.PostsDelete, label: '文章删除' }
    ]
  },
  {
    name: 'users',
    title: '用户管理',
    permissions: [
      { value: Permission.UsersRead, label: '用户读取' },
      { value: Permission.UsersWrite, label: '用户写入' },
      { value: Permission.UsersDelete, label: '用户删除' }
    ]
  },
  {
    name: 'system',
    title: '系统管理',
    permissions: [
      { value: Permission.Settings, label: '设置管理' },
      { value: Permission.SystemAdmin, label: '系统管理员' }
    ]
  }
]

// 模拟角色数据
const roles = ref([
  {
    id: '1',
    name: 'admin',
    description: '系统管理员，拥有所有权限',
    permissions: [
      Permission.Dashboard,
      Permission.PostsRead,
      Permission.PostsWrite,
      Permission.PostsDelete,
      Permission.UsersRead,
      Permission.UsersWrite,
      Permission.UsersDelete,
      Permission.Settings,
      Permission.SystemAdmin
    ],
    user_count: 2,
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: '2',
    name: 'moderator',
    description: '版主，负责内容审核和用户管理',
    permissions: [
      Permission.Dashboard,
      Permission.PostsRead,
      Permission.PostsWrite,
      Permission.PostsDelete,
      Permission.UsersRead,
      Permission.Settings
    ],
    user_count: 5,
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: '3',
    name: 'user',
    description: '普通用户，基础功能访问',
    permissions: [
      Permission.Dashboard,
      Permission.PostsRead,
      Permission.PostsWrite,
      Permission.Settings
    ],
    user_count: 15,
    created_at: '2024-01-01T00:00:00Z'
  }
])

// 模拟用户数据
const users = ref([
  {
    id: '1',
    username: 'admin',
    email: 'admin@example.com',
    role: 'admin',
    avatar: '',
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: '2',
    username: 'moderator1',
    email: 'moderator1@example.com',
    role: 'moderator',
    avatar: '',
    created_at: '2024-01-01T00:00:00Z'
  },
  {
    id: '3',
    username: 'user1',
    email: 'user1@example.com',
    role: 'user',
    avatar: '',
    created_at: '2024-01-01T00:00:00Z'
  }
])

// 过滤后的用户
const filteredUsers = computed(() => {
  if (!userSearchQuery.value) return users.value
  return users.value.filter(user => 
    user.username.toLowerCase().includes(userSearchQuery.value.toLowerCase()) ||
    user.email.toLowerCase().includes(userSearchQuery.value.toLowerCase())
  )
})

// 可用角色
const availableRoles = computed(() => roles.value)

// 获取角色类型
const getRoleType = (roleName: string) => {
  switch (roleName) {
    case 'admin': return 'danger'
    case 'moderator': return 'warning'
    case 'user': return 'info'
    default: return 'info'
  }
}

// 获取角色文本
const getRoleText = (roleName: string) => {
  switch (roleName) {
    case 'admin': return '管理员'
    case 'moderator': return '版主'
    case 'user': return '用户'
    default: return roleName
  }
}

// 获取权限文本
const getPermissionText = (permission: string) => {
  const allPermissions = permissionGroups.flatMap(group => group.permissions)
  const perm = allPermissions.find(p => p.value === permission)
  return perm ? perm.label : permission
}

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleDateString('zh-CN')
}

// 刷新角色
const refreshRoles = () => {
  loading.value = true
  setTimeout(() => {
    loading.value = false
    ElMessage.success('刷新成功')
  }, 1000)
}

// 编辑角色
const editRole = (role: any) => {
  editingRole.value = role
  Object.assign(roleForm, {
    id: role.id,
    name: role.name,
    description: role.description,
    permissions: [...(role.permissions || [])]
  })
  showCreateRoleDialog.value = true
}

// 删除角色
const deleteRole = async (role: any) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除角色"${getRoleText(role.name)}"吗？`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    ElMessage.success('删除成功')
    // 重新加载角色数据
  } catch {
    // 用户取消删除
  }
}

// 分配用户
const assignUsers = (role: any) => {
  ElMessage.info(`为角色"${getRoleText(role.name)}"分配用户`)
}

// 保存角色
const saveRole = async () => {
  saving.value = true
  try {
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
    
    ElMessage.success(editingRole.value ? '保存成功' : '创建成功')
    showCreateRoleDialog.value = false
    
    // 重新加载角色数据
    if (editingRole.value) {
      Object.assign(editingRole.value, { ...roleForm })
    } else {
      // 添加新角色
      const newRole = {
        ...roleForm,
        id: Date.now().toString(),
        user_count: 0,
        created_at: new Date().toISOString()
      }
      roles.value.push(newRole)
    }
  } catch (error) {
    ElMessage.error('操作失败')
  } finally {
    saving.value = false
  }
}

// 修改用户角色
const changeUserRole = (user: any) => {
  selectedUser.value = user
  selectedUserRole.value = user.role
  showUserRoleDialog.value = true
}

// 更新用户角色
const updateUserRole = async () => {
  if (!selectedUser.value || !selectedUserRole.value) {
    ElMessage.warning('请选择用户和角色')
    return
  }
  
  updatingRole.value = true
  try {
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
    
    // 更新用户角色
    selectedUser.value.role = selectedUserRole.value
    
    ElMessage.success('角色修改成功')
    showUserRoleDialog.value = false
  } catch (error) {
    ElMessage.error('操作失败')
  } finally {
    updatingRole.value = false
  }
}

// 用户搜索
const handleUserSearch = () => {
  // 搜索逻辑已在computed中实现
}

onMounted(() => {
  // 加载数据
  loading.value = true
  usersLoading.value = true
  
  setTimeout(() => {
    loading.value = false
    usersLoading.value = false
  }, 1000)
})
</script>

<style scoped>
.permission-management-container {
  min-height: 100vh;
  background-color: #f5f5f5;
}

.page-header {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  padding: 24px 0;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
}

.header-title {
  font-size: 24px;
  font-weight: 600;
  color: #333;
  margin: 0 0 4px 0;
}

.header-subtitle {
  font-size: 14px;
  color: #666;
  margin: 0;
}

.permission-main {
  padding: 24px 0;
}

.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.role-card,
.user-role-card {
  border-radius: 12px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.role-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.permissions-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.permission-tag {
  font-size: 11px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.username {
  font-weight: 500;
}

.search-section {
  margin-bottom: 16px;
}

.permissions-section {
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  padding: 16px;
  max-height: 300px;
  overflow-y: auto;
}

.permission-group {
  margin-bottom: 20px;
}

.permission-group:last-child {
  margin-bottom: 0;
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
  margin: 0 0 12px 0;
  padding-bottom: 8px;
  border-bottom: 1px solid #f0f0f0;
}

.permission-items {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.user-role-dialog {
  padding: 16px 0;
}

.user-role-dialog .user-info {
  margin-bottom: 24px;
  padding: 16px;
  background-color: #f8f9fa;
  border-radius: 8px;
}

.user-details h4 {
  margin: 0 0 4px 0;
  font-size: 16px;
  font-weight: 600;
}

.user-details p {
  margin: 0;
  color: #666;
  font-size: 14px;
}

.role-option {
  display: flex;
  align-items: center;
  gap: 8px;
}

.role-desc {
  color: #666;
  font-size: 12px;
}

.dialog-footer {
  text-align: right;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .header-content {
    flex-direction: column;
    gap: 16px;
    text-align: center;
  }
  
  .header-title {
    font-size: 20px;
  }
  
  .card-header {
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  
  .header-actions {
    width: 100%;
    justify-content: flex-end;
  }
  
  .permission-items {
    flex-direction: column;
    gap: 8px;
  }
}
</style>
