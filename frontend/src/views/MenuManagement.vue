<template>
  <div class="menu-management-container">
    <!-- 导航栏 -->
    <AppNavbar />
    
    <!-- 页面头部 -->
    <header class="page-header">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">菜单管理</h1>
          <p class="header-subtitle">管理系统菜单结构和权限配置</p>
        </div>
        <div class="header-right">
          <el-button type="primary" @click="showCreateDialog = true" size="large">
            <el-icon><Plus /></el-icon>
            新建菜单
          </el-button>
        </div>
      </div>
    </header>

    <!-- 主要内容 -->
    <main class="menu-main">
      <div class="container">
        <!-- 菜单树形表格 -->
        <el-card class="menu-card">
          <template #header>
            <div class="card-header">
              <span>菜单结构</span>
              <div class="header-actions">
                <el-button @click="expandAll" size="small">
                  <el-icon><Expand /></el-icon>
                  展开全部
                </el-button>
                <el-button @click="collapseAll" size="small">
                  <el-icon><Fold /></el-icon>
                  折叠全部
                </el-button>
              </div>
            </div>
          </template>

          <el-table
            :data="menuTree"
            :size="layoutConfig.table.size"
            :stripe="layoutConfig.table.stripe"
            :border="layoutConfig.table.border"
            row-key="id"
            :tree-props="{ children: 'children', hasChildren: 'hasChildren' }"
            v-loading="loading"
          >
            <el-table-column prop="title" label="菜单名称" min-width="200">
              <template #default="{ row }">
                <div class="menu-item">
                  <el-icon v-if="row.icon" class="menu-icon">
                    <component :is="getIcon(row.icon)" />
                  </el-icon>
                  <span class="menu-title">{{ row.title }}</span>
                  <el-tag v-if="row.path" size="small" type="info">{{ row.path }}</el-tag>
                </div>
              </template>
            </el-table-column>
            
            <el-table-column prop="path" label="路径" width="200">
              <template #default="{ row }">
                <span v-if="row.path" class="path-text">{{ row.path }}</span>
                <span v-else class="path-text text-muted">-</span>
              </template>
            </el-table-column>
            
            <el-table-column prop="permissions" label="权限" width="300">
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
                  <span v-if="!row.permissions || row.permissions.length === 0" class="text-muted">
                    无权限要求
                  </span>
                </div>
              </template>
            </el-table-column>
            
            <el-table-column prop="sort_order" label="排序" width="80" align="center">
              <template #default="{ row }">
                <span>{{ row.sort_order || 0 }}</span>
              </template>
            </el-table-column>
            
            <el-table-column prop="enabled" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
                  {{ row.enabled ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button-group>
                  <el-button size="small" @click="editMenu(row)">
                    <el-icon><Edit /></el-icon>
                  </el-button>
                  <el-button size="small" @click="addSubMenu(row)">
                    <el-icon><Plus /></el-icon>
                  </el-button>
                  <el-button 
                    size="small" 
                    type="danger" 
                    @click="deleteMenu(row)"
                    :disabled="row.children && row.children.length > 0"
                  >
                    <el-icon><Delete /></el-icon>
                  </el-button>
                </el-button-group>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </div>
    </main>

    <!-- 创建/编辑菜单对话框 -->
    <el-dialog
      v-model="showCreateDialog"
      :title="editingMenu ? '编辑菜单' : '新建菜单'"
      width="600px"
      :close-on-click-modal="false"
    >
      <el-form
        ref="formRef"
        :model="menuForm"
        :rules="formRules"
        label-width="100px"
      >
        <el-form-item label="菜单名称" prop="title">
          <el-input v-model="menuForm.title" placeholder="请输入菜单名称" />
        </el-form-item>
        
        <el-form-item label="菜单路径" prop="path">
          <el-input v-model="menuForm.path" placeholder="请输入菜单路径，如：/dashboard" />
        </el-form-item>
        
        <el-form-item label="菜单图标" prop="icon">
          <el-select v-model="menuForm.icon" placeholder="请选择图标" clearable>
            <el-option
              v-for="icon in availableIcons"
              :key="icon.value"
              :label="icon.label"
              :value="icon.value"
            >
              <div class="icon-option">
                <el-icon><component :is="icon.value" /></el-icon>
                <span>{{ icon.label }}</span>
              </div>
            </el-option>
          </el-select>
        </el-form-item>
        
        <el-form-item label="上级菜单" prop="parent_id">
          <el-tree-select
            v-model="menuForm.parent_id"
            :data="menuOptions"
            :props="{ label: 'title', value: 'id', children: 'children' }"
            placeholder="请选择上级菜单"
            clearable
            check-strictly
          />
        </el-form-item>
        
        <el-form-item label="排序" prop="sort_order">
          <el-input-number v-model="menuForm.sort_order" :min="0" :max="999" />
        </el-form-item>
        
        <el-form-item label="状态" prop="enabled">
          <el-switch v-model="menuForm.enabled" />
        </el-form-item>
        
        <el-form-item label="所需权限">
          <el-checkbox-group v-model="menuForm.permissions">
            <el-checkbox
              v-for="perm in availablePermissions"
              :key="perm.value"
              :label="perm.value"
            >
              {{ perm.label }}
            </el-checkbox>
          </el-checkbox-group>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="showCreateDialog = false">取消</el-button>
          <el-button type="primary" @click="saveMenu" :loading="saving">
            {{ editingMenu ? '保存' : '创建' }}
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
  Plus, Edit, Delete, Expand, Fold,
  Monitor, DataBoard, Document, User, Setting, List, UserFilled
} from '@element-plus/icons-vue'
import { useLayoutConfig } from '@/utils/responsive'
import { Permission } from '@/types'
import AppNavbar from '@/components/AppNavbar.vue'

const { layoutConfig } = useLayoutConfig()

// 响应式数据
const loading = ref(false)
const saving = ref(false)
const showCreateDialog = ref(false)
const editingMenu = ref<any>(null)

// 菜单表单
const menuForm = reactive({
  id: '',
  title: '',
  path: '',
  icon: '',
  parent_id: null as string | null,
  sort_order: 0,
  enabled: true,
  permissions: [] as string[]
})

// 表单验证规则
const formRules = {
  title: [
    { required: true, message: '请输入菜单名称', trigger: 'blur' }
  ],
  path: [
    { required: true, message: '请输入菜单路径', trigger: 'blur' }
  ]
}

// 可用图标
const availableIcons = [
  { value: 'Monitor', label: '显示器' },
  { value: 'DataBoard', label: '仪表盘' },
  { value: 'Document', label: '文档' },
  { value: 'User', label: '用户' },
  { value: 'Setting', label: '设置' },
  { value: 'List', label: '列表' },
  { value: 'UserFilled', label: '用户填充' }
]

// 可用权限
const availablePermissions = [
  { value: Permission.Dashboard, label: '仪表盘' },
  { value: Permission.PostsRead, label: '文章读取' },
  { value: Permission.PostsWrite, label: '文章写入' },
  { value: Permission.PostsDelete, label: '文章删除' },
  { value: Permission.UsersRead, label: '用户读取' },
  { value: Permission.UsersWrite, label: '用户写入' },
  { value: Permission.UsersDelete, label: '用户删除' },
  { value: Permission.Settings, label: '设置' },
  { value: Permission.SystemAdmin, label: '系统管理' }
]

// 模拟菜单数据
const menuTree = ref([
  {
    id: '1',
    title: '仪表盘',
    path: '/dashboard',
    icon: 'DataBoard',
    sort_order: 1,
    enabled: true,
    permissions: [Permission.Dashboard],
    children: []
  },
  {
    id: '2',
    title: '文章管理',
    path: '/posts',
    icon: 'Document',
    sort_order: 2,
    enabled: true,
    permissions: [Permission.PostsRead],
    children: [
      {
        id: '2-1',
        title: '文章列表',
        path: '/posts',
        icon: 'List',
        sort_order: 1,
        enabled: true,
        permissions: [Permission.PostsRead],
        children: []
      },
      {
        id: '2-2',
        title: '创建文章',
        path: '/posts/create',
        icon: 'Plus',
        sort_order: 2,
        enabled: true,
        permissions: [Permission.PostsWrite],
        children: []
      }
    ]
  },
  {
    id: '3',
    title: '用户管理',
    path: '/users',
    icon: 'User',
    sort_order: 3,
    enabled: true,
    permissions: [Permission.UsersRead],
    children: []
  },
  {
    id: '4',
    title: '系统设置',
    path: '/settings',
    icon: 'Setting',
    sort_order: 4,
    enabled: true,
    permissions: [Permission.Settings],
    children: [
      {
        id: '4-1',
        title: '个人资料',
        path: '/profile',
        icon: 'UserFilled',
        sort_order: 1,
        enabled: true,
        permissions: [Permission.Settings],
        children: []
      }
    ]
  }
])

// 菜单选项（用于上级菜单选择）
const menuOptions = computed(() => {
  return [{ id: '', title: '顶级菜单', children: [] }, ...menuTree.value]
})

// 获取图标组件
const getIcon = (iconName: string) => {
  const iconMap: Record<string, any> = {
    Monitor,
    DataBoard,
    Document,
    User,
    Setting,
    List,
    UserFilled,
    Plus
  }
  return iconMap[iconName] || Monitor
}

// 获取权限文本
const getPermissionText = (permission: string) => {
  const perm = availablePermissions.find(p => p.value === permission)
  return perm ? perm.label : permission
}

// 展开全部
const expandAll = () => {
  // 这里需要实现展开逻辑
  ElMessage.success('展开全部菜单')
}

// 折叠全部
const collapseAll = () => {
  // 这里需要实现折叠逻辑
  ElMessage.success('折叠全部菜单')
}

// 编辑菜单
const editMenu = (menu: any) => {
  editingMenu.value = menu
  Object.assign(menuForm, {
    id: menu.id,
    title: menu.title,
    path: menu.path,
    icon: menu.icon,
    parent_id: menu.parent_id,
    sort_order: menu.sort_order,
    enabled: menu.enabled,
    permissions: [...(menu.permissions || [])]
  })
  showCreateDialog.value = true
}

// 添加子菜单
const addSubMenu = (parentMenu: any) => {
  editingMenu.value = null
  Object.assign(menuForm, {
    id: '',
    title: '',
    path: '',
    icon: '',
    parent_id: parentMenu.id,
    sort_order: 0,
    enabled: true,
    permissions: []
  })
  showCreateDialog.value = true
}

// 删除菜单
const deleteMenu = async (menu: any) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除菜单"${menu.title}"吗？`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    // 这里调用删除API
    ElMessage.success('删除成功')
    // 重新加载菜单数据
  } catch {
    // 用户取消删除
  }
}

// 保存菜单
const saveMenu = async () => {
  saving.value = true
  try {
    // 这里调用保存API
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
    
    ElMessage.success(editingMenu.value ? '保存成功' : '创建成功')
    showCreateDialog.value = false
    
    // 重新加载菜单数据
    if (editingMenu.value) {
      Object.assign(editingMenu.value, { ...menuForm })
    } else {
      // 添加新菜单到树中
      const newMenu = { ...menuForm, id: Date.now().toString(), children: [] }
      if (menuForm.parent_id) {
        // 添加到父菜单
        const parent = findMenuById(menuTree.value, menuForm.parent_id)
        if (parent) {
          parent.children = parent.children || []
          parent.children.push(newMenu)
        }
      } else {
        // 添加到顶级
        menuTree.value.push(newMenu)
      }
    }
  } catch (error) {
    ElMessage.error('操作失败')
  } finally {
    saving.value = false
  }
}

// 根据ID查找菜单
const findMenuById = (menus: any[], id: string): any => {
  for (const menu of menus) {
    if (menu.id === id) return menu
    if (menu.children) {
      const found = findMenuById(menu.children, id)
      if (found) return found
    }
  }
  return null
}

onMounted(() => {
  // 加载菜单数据
  loading.value = true
  setTimeout(() => {
    loading.value = false
  }, 1000)
})
</script>

<style scoped>
.menu-management-container {
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

.menu-main {
  padding: 24px 0;
}

.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px;
}

.menu-card {
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

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.menu-icon {
  color: #409eff;
}

.menu-title {
  font-weight: 500;
}

.path-text {
  font-family: monospace;
  font-size: 12px;
}

.text-muted {
  color: #999;
}

.permissions-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.permission-tag {
  font-size: 11px;
}

.icon-option {
  display: flex;
  align-items: center;
  gap: 8px;
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
}
</style>
