<template>
  <div class="settings-container">
    <!-- 导航栏 -->
    <AppNavbar />
    
    <!-- 页面头部 -->
    <header class="page-header">
      <div class="header-content">
        <div class="header-left">
          <h1 class="header-title">系统设置</h1>
        </div>
      </div>
    </header>

    <!-- 主要内容 -->
    <main class="settings-main">
      <div class="container">
        <div class="settings-grid">
          <!-- 主题设置 -->
          <el-card class="theme-card">
            <template #header>
              <span>主题设置</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>主题模式</span>
                <span class="setting-desc">选择您喜欢的主题模式</span>
              </div>
              <div class="setting-control">
                <el-radio-group v-model="settings.theme" @change="handleThemeChange">
                  <el-radio-button value="light">
                    <el-icon><Sunny /></el-icon>
                    浅色模式
                  </el-radio-button>
                  <el-radio-button value="dark">
                    <el-icon><Moon /></el-icon>
                    深色模式
                  </el-radio-button>
                  <el-radio-button value="auto">
                    <el-icon><Monitor /></el-icon>
                    跟随系统
                  </el-radio-button>
                </el-radio-group>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>主题色彩</span>
                <span class="setting-desc">选择您喜欢的主题色彩</span>
              </div>
              <div class="setting-control">
                <el-color-picker 
                  v-model="settings.primaryColor" 
                  @change="handleColorChange"
                  show-alpha
                />
              </div>
            </div>
          </el-card>

          <!-- 通知设置 -->
          <el-card class="notification-card">
            <template #header>
              <span>通知设置</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>邮件通知</span>
                <span class="setting-desc">接收邮件通知</span>
              </div>
              <div class="setting-control">
                <el-switch 
                  v-model="settings.emailNotifications" 
                  @change="handleNotificationChange"
                />
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>浏览器通知</span>
                <span class="setting-desc">接收浏览器推送通知</span>
              </div>
              <div class="setting-control">
                <el-switch 
                  v-model="settings.browserNotifications" 
                  @change="handleNotificationChange"
                />
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>通知类型</span>
                <span class="setting-desc">选择接收的通知类型</span>
              </div>
              <div class="setting-control">
                <el-checkbox-group v-model="settings.notificationTypes" @change="handleNotificationChange">
                  <el-checkbox label="posts">文章相关</el-checkbox>
                  <el-checkbox label="system">系统消息</el-checkbox>
                  <el-checkbox label="security">安全提醒</el-checkbox>
                </el-checkbox-group>
              </div>
            </div>
          </el-card>

          <!-- 安全设置 -->
          <el-card class="security-card">
            <template #header>
              <span>安全设置</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>两步验证</span>
                <span class="setting-desc">启用两步验证提高账户安全性</span>
              </div>
              <div class="setting-control">
                <el-switch 
                  v-model="settings.twoFactorAuth" 
                  @change="handleSecurityChange"
                />
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>登录设备管理</span>
                <span class="setting-desc">管理已登录的设备</span>
              </div>
              <div class="setting-control">
                <el-button size="small" @click="showDeviceManager = true">
                  查看设备
                </el-button>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>会话超时</span>
                <span class="setting-desc">设置自动登出时间</span>
              </div>
              <div class="setting-control">
                <el-select v-model="settings.sessionTimeout" @change="handleSecurityChange">
                  <el-option label="30分钟" value="30" />
                  <el-option label="1小时" value="60" />
                  <el-option label="4小时" value="240" />
                  <el-option label="24小时" value="1440" />
                  <el-option label="永不" value="0" />
                </el-select>
              </div>
            </div>
          </el-card>

          <!-- 隐私设置 -->
          <el-card class="privacy-card">
            <template #header>
              <span>隐私设置</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>个人资料可见性</span>
                <span class="setting-desc">设置个人资料的可见范围</span>
              </div>
              <div class="setting-control">
                <el-select v-model="settings.profileVisibility" @change="handlePrivacyChange">
                  <el-option label="公开" value="public" />
                  <el-option label="仅好友" value="friends" />
                  <el-option label="私密" value="private" />
                </el-select>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>文章可见性</span>
                <span class="setting-desc">设置默认的文章可见性</span>
              </div>
              <div class="setting-control">
                <el-select v-model="settings.defaultPostVisibility" @change="handlePrivacyChange">
                  <el-option label="公开" value="public" />
                  <el-option label="仅好友" value="friends" />
                  <el-option label="私密" value="private" />
                </el-select>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>数据收集</span>
                <span class="setting-desc">允许收集使用数据以改善服务</span>
              </div>
              <div class="setting-control">
                <el-switch 
                  v-model="settings.dataCollection" 
                  @change="handlePrivacyChange"
                />
              </div>
            </div>
          </el-card>

          <!-- 语言设置 -->
          <el-card class="language-card">
            <template #header>
              <span>语言设置</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>界面语言</span>
                <span class="setting-desc">选择界面显示语言</span>
              </div>
              <div class="setting-control">
                <el-select v-model="settings.language" @change="handleLanguageChange">
                  <el-option label="简体中文" value="zh-CN" />
                  <el-option label="English" value="en-US" />
                  <el-option label="日本語" value="ja-JP" />
                </el-select>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>时区设置</span>
                <span class="setting-desc">设置您的时区</span>
              </div>
              <div class="setting-control">
                <el-select v-model="settings.timezone" @change="handleLanguageChange">
                  <el-option label="UTC+8 (北京时间)" value="Asia/Shanghai" />
                  <el-option label="UTC+0 (格林威治时间)" value="UTC" />
                  <el-option label="UTC-5 (美国东部时间)" value="America/New_York" />
                  <el-option label="UTC+1 (欧洲中部时间)" value="Europe/Paris" />
                </el-select>
              </div>
            </div>
          </el-card>

          <!-- 数据管理 -->
          <el-card class="data-card">
            <template #header>
              <span>数据管理</span>
            </template>

            <div class="setting-item">
              <div class="setting-label">
                <span>导出数据</span>
                <span class="setting-desc">导出您的个人数据</span>
              </div>
              <div class="setting-control">
                <el-button size="small" @click="exportData">
                  导出数据
                </el-button>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <span>删除账户</span>
                <span class="setting-desc">永久删除您的账户和所有数据</span>
              </div>
              <div class="setting-control">
                <el-button size="small" type="danger" @click="showDeleteConfirm = true">
                  删除账户
                </el-button>
              </div>
            </div>
          </el-card>
        </div>

        <!-- 保存按钮 -->
        <div class="save-section">
          <el-button type="primary" size="large" @click="saveSettings" :loading="saving">
            保存设置
          </el-button>
          <el-button size="large" @click="resetSettings">
            重置设置
          </el-button>
        </div>
      </div>
    </main>

    <!-- 设备管理对话框 -->
    <el-dialog v-model="showDeviceManager" title="登录设备管理" width="600px">
      <div class="device-list">
        <div v-for="device in devices" :key="device.id" class="device-item">
          <div class="device-info">
            <div class="device-name">{{ device.name }}</div>
            <div class="device-details">
              {{ device.browser }} - {{ device.location }} - {{ formatDate(device.lastLogin) }}
            </div>
          </div>
          <div class="device-actions">
            <el-button size="small" type="danger" @click="logoutDevice(device.id)">
              登出
            </el-button>
          </div>
        </div>
      </div>
    </el-dialog>

    <!-- 删除确认对话框 -->
    <el-dialog v-model="showDeleteConfirm" title="确认删除账户" width="400px">
      <div class="delete-warning">
        <el-alert
          title="警告"
          type="error"
          description="此操作将永久删除您的账户和所有相关数据，无法恢复。"
          show-icon
        />
      </div>
      <template #footer>
        <el-button @click="showDeleteConfirm = false">取消</el-button>
        <el-button type="danger" @click="deleteAccount">确认删除</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Sunny, Moon, Monitor } from '@element-plus/icons-vue'
import { apiService } from '@/api'
import AppNavbar from '@/components/AppNavbar.vue'

// 响应式数据
const saving = ref(false)
const showDeviceManager = ref(false)
const showDeleteConfirm = ref(false)

// 设置数据
const settings = reactive({
  theme: 'light',
  primaryColor: '#409eff',
  emailNotifications: true,
  browserNotifications: false,
  notificationTypes: ['posts', 'system'],
  twoFactorAuth: false,
  sessionTimeout: '60',
  profileVisibility: 'public',
  defaultPostVisibility: 'public',
  dataCollection: true,
  language: 'zh-CN',
  timezone: 'Asia/Shanghai'
})

// 设备列表
const devices = ref([
  {
    id: '1',
    name: 'Windows 10 - Chrome',
    browser: 'Chrome 120.0',
    location: '北京, 中国',
    lastLogin: '2024-01-15T10:30:00Z'
  },
  {
    id: '2',
    name: 'iPhone - Safari',
    browser: 'Safari 17.0',
    location: '上海, 中国',
    lastLogin: '2024-01-14T15:20:00Z'
  }
])

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

// 加载设置
const loadSettings = async () => {
  try {
    const response = await apiService.getSettings()
    if (response.success && response.data) {
      // 将后端数据转换为前端格式
      Object.assign(settings, {
        theme: response.data.theme,
        primaryColor: response.data.primary_color,
        emailNotifications: response.data.email_notifications,
        browserNotifications: response.data.browser_notifications,
        notificationTypes: response.data.notification_types,
        twoFactorAuth: response.data.two_factor_auth,
        sessionTimeout: response.data.session_timeout.toString(),
        profileVisibility: response.data.profile_visibility,
        defaultPostVisibility: response.data.default_post_visibility,
        dataCollection: response.data.data_collection,
        language: response.data.language,
        timezone: response.data.timezone
      })
    }
  } catch (error) {
    console.error('加载设置失败:', error)
    ElMessage.error('加载设置失败')
  }
}

// 加载设备列表
const loadDevices = async () => {
  try {
    const response = await apiService.getUserDevices()
    if (response.success && response.data) {
      devices.value = response.data
    }
  } catch (error) {
    console.error('加载设备列表失败:', error)
    ElMessage.error('加载设备列表失败')
  }
}

// 保存设置
const saveSettings = async () => {
  saving.value = true
  try {
    // 将前端数据转换为后端格式
    const settingsData = {
      theme: settings.theme,
      primary_color: settings.primaryColor,
      email_notifications: settings.emailNotifications,
      browser_notifications: settings.browserNotifications,
      notification_types: settings.notificationTypes,
      two_factor_auth: settings.twoFactorAuth,
      session_timeout: parseInt(settings.sessionTimeout),
      profile_visibility: settings.profileVisibility,
      default_post_visibility: settings.defaultPostVisibility,
      data_collection: settings.dataCollection,
      language: settings.language,
      timezone: settings.timezone
    }
    
    const response = await apiService.updateSettings(settingsData)
    if (response.success) {
      ElMessage.success('设置保存成功')
      // 应用主题设置
      applyThemeSettings()
    } else {
      ElMessage.error(response.message || '保存失败')
    }
  } catch (error) {
    console.error('保存设置失败:', error)
    ElMessage.error('保存失败')
  } finally {
    saving.value = false
  }
}

// 重置设置
const resetSettings = async () => {
  try {
    await ElMessageBox.confirm('确定要重置所有设置吗？', '确认重置', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    // 重置为默认设置
    Object.assign(settings, {
      theme: 'light',
      primaryColor: '#409eff',
      emailNotifications: true,
      browserNotifications: false,
      notificationTypes: ['posts', 'system'],
      twoFactorAuth: false,
      sessionTimeout: '60',
      profileVisibility: 'public',
      defaultPostVisibility: 'public',
      dataCollection: true,
      language: 'zh-CN',
      timezone: 'Asia/Shanghai'
    })
    
    ElMessage.success('设置已重置')
  } catch {
    // 用户取消
  }
}

// 导出数据
const exportData = async () => {
  try {
    const user = JSON.parse(localStorage.getItem('user') || '{}')
    const exportRequest = {
      email: user.email || '',
      data_types: ['profile', 'posts', 'settings']
    }
    
    const response = await apiService.exportData(exportRequest)
    if (response.success) {
      ElMessage.success('数据导出请求已提交，请检查您的邮箱')
    } else {
      ElMessage.error(response.message || '导出失败')
    }
  } catch (error) {
    console.error('导出数据失败:', error)
    ElMessage.error('导出失败')
  }
}

// 登出设备
const logoutDevice = async (deviceId: string) => {
  try {
    await ElMessageBox.confirm('确定要登出此设备吗？', '确认登出', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    const response = await apiService.logoutDevice(deviceId)
    if (response.success) {
      devices.value = devices.value.filter(d => d.id !== deviceId)
      ElMessage.success('设备已登出')
    } else {
      ElMessage.error(response.message || '登出失败')
    }
  } catch (error) {
    if (error !== 'cancel') {
      console.error('登出设备失败:', error)
      ElMessage.error('登出失败')
    }
  }
}

// 删除账户
const deleteAccount = async () => {
  try {
    const { value: password } = await ElMessageBox.prompt(
      '请输入您的密码确认删除：',
      '确认删除账户',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'error',
        inputType: 'password',
        inputValidator: (value) => {
          if (!value) {
            return '请输入密码'
          }
          return true
        }
      }
    )
    
    const { value: confirmation } = await ElMessageBox.prompt(
      '请输入"DELETE"确认删除：',
      '最终确认',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'error',
        inputPattern: /^DELETE$/,
        inputErrorMessage: '请输入DELETE确认删除'
      }
    )
    
    const deleteRequest = {
      password: password,
      confirmation: confirmation
    }
    
    const response = await apiService.deleteAccount(deleteRequest)
    if (response.success) {
      ElMessage.success('账户删除成功')
      // 清除本地存储并跳转到登录页面
      localStorage.removeItem('token')
      localStorage.removeItem('user')
      window.location.href = '/login'
    } else {
      ElMessage.error(response.message || '删除失败')
    }
  } catch (error) {
    if (error !== 'cancel') {
      console.error('删除账户失败:', error)
      ElMessage.error('删除失败')
    }
  }
}

// 应用主题设置
const applyThemeSettings = () => {
  // 应用主题模式
  if (settings.theme === 'dark') {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
  
  // 应用主题色彩
  if (settings.primaryColor) {
    document.documentElement.style.setProperty('--el-color-primary', settings.primaryColor)
  }
}

// 主题变更
const handleThemeChange = (value: string) => {
  console.log('主题变更:', value)
  applyThemeSettings()
}

// 颜色变更
const handleColorChange = (value: string) => {
  console.log('颜色变更:', value)
  applyThemeSettings()
}

// 通知设置变更
const handleNotificationChange = () => {
  console.log('通知设置变更:', settings)
}

// 安全设置变更
const handleSecurityChange = () => {
  console.log('安全设置变更:', settings)
}

// 隐私设置变更
const handlePrivacyChange = () => {
  console.log('隐私设置变更:', settings)
}

// 语言设置变更
const handleLanguageChange = () => {
  console.log('语言设置变更:', settings)
}

onMounted(() => {
  loadSettings()
  loadDevices()
})
</script>

<style scoped>
.settings-container {
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
.settings-main {
  padding: 24px 0;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
}

/* 网格布局 */
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
  gap: 24px;
  margin-bottom: 32px;
}

/* 设置项样式 */
.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 16px 0;
  border-bottom: 1px solid #f0f0f0;
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-label {
  flex: 1;
  margin-right: 24px;
}

.setting-label span:first-child {
  display: block;
  font-weight: 500;
  color: #333;
  margin-bottom: 4px;
}

.setting-desc {
  font-size: 12px;
  color: #999;
}

.setting-control {
  flex-shrink: 0;
}

/* 保存区域 */
.save-section {
  display: flex;
  justify-content: center;
  gap: 16px;
  padding: 24px 0;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* 设备列表 */
.device-list {
  max-height: 400px;
  overflow-y: auto;
}

.device-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #f0f0f0;
}

.device-item:last-child {
  border-bottom: none;
}

.device-info {
  flex: 1;
}

.device-name {
  font-weight: 500;
  margin-bottom: 4px;
}

.device-details {
  font-size: 12px;
  color: #999;
}

.device-actions {
  flex-shrink: 0;
}

/* 删除警告 */
.delete-warning {
  margin-bottom: 16px;
}

/* 移动端优化 */
@media (max-width: 767px) {
  .header-content {
    padding: 0 16px;
  }
  
  .header-title {
    font-size: 20px;
  }
  
  .settings-main {
    padding: 16px 0;
  }
  
  .container {
    padding: 0 16px;
  }
  
  .settings-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .setting-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  
  .setting-label {
    margin-right: 0;
  }
  
  .save-section {
    flex-direction: column;
    gap: 12px;
  }
}

/* 平板端优化 */
@media (min-width: 768px) and (max-width: 1023px) {
  .settings-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
