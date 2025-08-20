<template>
  <div class="users-container">
    <AppNavbar />
    
    <header class="page-header">
      <div class="header-content">
        <h1 class="page-title">用户管理</h1>
        <div class="header-actions">
          <el-button 
            type="primary" 
            @click="showCreateDialog = true"
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
        <!-- 搜索和筛选 -->
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
            <el-option label="管理员" value="admin" />
            <el-option label="版主" value="moderator" />
            <el-option label="用户" value="user" />
          </el-select>
        </div>

        <!-- 用户列表 -->
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

        <!-- 分页 -->
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
            <el-option label="用户" value="user" />
            <el-option label="版主" value="moderator" />
            <el-option label="管理员" value="admin" />
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
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus } from '@element-plus/icons-vue'
import AppNavbar from '@/components/AppNavbar.vue'
import { useAuthStore } from '@/stores/auth'
import { Permission, UserRole } from '@/types'
import type { User } from '@/types'

const authStore = useAuthStore()

// 响应式数据
const loading = ref(false)
const users = ref<User[]>([])
const searchQuery = ref('')
const roleFilter = ref('')
const currentPage = ref(1)
const pageSize = ref(20)
const totalUsers = ref(0)

// 编辑角色相关
const showRoleDialog = ref(false)
const editingUser = ref<User>({} as User)
const newRole = ref('')
const updating = ref(false)

// 计算属性
const filteredUsers = computed(() => {
  let filtered = users.value

  // 搜索过滤
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

// 获取角色标签类型
const getRoleTagType = (role: string) => {
  switch (role) {
    case UserRole.Admin:
      return 'danger'
    case UserRole.Moderator:
      return 'warning'
    case UserRole.User:
      return 'info'
    default:
      return 'info'
  }
}

// 获取角色文本
const getRoleText = (role: string) => {
  switch (role) {
    case UserRole.Admin:
      return '管理员'
    case UserRole.Moderator:
      return '版主'
    case UserRole.User:
      return '用户'
    default:
      return '用户'
  }
}

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

// 搜索处理
const handleSearch = () => {
  currentPage.value = 1
  // 这里可以调用API进行搜索
}

// 分页处理
const handleSizeChange = (size: number) => {
  pageSize.value = size
  currentPage.value = 1
  fetchUsers()
}

const handleCurrentChange = (page: number) => {
  currentPage.value = page
  fetchUsers()
}

// 获取用户列表
const fetchUsers = async () => {
  loading.value = true
  try {
    // 这里应该调用API获取用户列表
    // const response = await apiService.getUsers({
    //   page: currentPage.value,
    //   size: pageSize.value,
    //   search: searchQuery.value,
    //   role: roleFilter.value
    // })
    // users.value = response.data.users
    // totalUsers.value = response.data.total

    // 模拟数据
    users.value = [
      {
        id: '1',
        username: 'admin',
        email: 'admin@example.com',
        role: UserRole.Admin,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z'
      },
      {
        id: '2',
        username: 'moderator',
        email: 'moderator@example.com',
        role: UserRole.Moderator,
        created_at: '2024-01-02T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z'
      },
      {
        id: '3',
        username: 'user1',
        email: 'user1@example.com',
        role: UserRole.User,
        created_at: '2024-01-03T00:00:00Z',
        updated_at: '2024-01-03T00:00:00Z'
      }
    ]
    totalUsers.value = users.value.length
  } catch (error) {
    ElMessage.error('获取用户列表失败')
  } finally {
    loading.value = false
  }
}

// 编辑用户角色
const editUserRole = (user: User) => {
  editingUser.value = { ...user }
  newRole.value = user.role
  showRoleDialog.value = true
}

// 更新用户角色
const updateUserRole = async () => {
  if (!newRole.value) {
    ElMessage.warning('请选择新角色')
    return
  }

  updating.value = true
  try {
    // 这里应该调用API更新用户角色
    // await apiService.updateUserRole(editingUser.value.id, newRole.value)
    
    // 更新本地数据
    const userIndex = users.value.findIndex(u => u.id === editingUser.value.id)
    if (userIndex !== -1) {
      users.value[userIndex].role = newRole.value
    }

    ElMessage.success('角色更新成功')
    showRoleDialog.value = false
  } catch (error) {
    ElMessage.error('角色更新失败')
  } finally {
    updating.value = false
  }
}

// 删除用户
const deleteUser = async (user: User) => {
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

    // 这里应该调用API删除用户
    // await apiService.deleteUser(user.id)
    
    // 更新本地数据
    users.value = users.value.filter(u => u.id !== user.id)
    totalUsers.value--

    ElMessage.success('用户删除成功')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除用户失败')
    }
  }
}

// 组件挂载时获取数据
onMounted(() => {
  fetchUsers()
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
  margin-top: 64px;
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
