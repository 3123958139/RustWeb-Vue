<!--
  发动机测控应用（fj200c_main）根组件

  - 全局导航条（AppNavbar），#actions 插槽放三个操作按钮：
    保存数据（CSV 录制切换）、模拟运行/停止、主题切换
  - 按钮状态由 dashboard store 驱动（WS 事件更新 store）
  - 组件挂载时初始化认证状态与主题
-->
<script lang="ts" setup>
import {computed, onMounted} from 'vue'
import {useRoute} from 'vue-router'
import {ElMessage, ElMessageBox} from 'element-plus'
import {useAuthStore} from '@/stores/auth'
import {useDashboardStore} from '@/fj200c_main/store/dashboard'
import {useBackendPorts} from '@/fj200c_main/composables/useBackendPorts'
import {useTheme} from '@/fj200c_main/composables/useTheme'
import {fj200cMainApi} from '@/api'
import {AppNavbar} from '@shared'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'

/** 路由实例，用于判断当前是否处于登录页 */
const route = useRoute()
/** 登录页不显示导航条，其余页面全局显示 */
const isLoginPage = computed(() => route.path.startsWith('/login'))

/** 认证 Store 实例 */
const authStore = useAuthStore()
/** 仪表盘 Store（保存数据/模拟/主题三态由 WS 事件更新） */
const dashboardStore = useDashboardStore()
/** 主题组合式函数（应用本地保存的主题） */
const {isDark, toggle: toggleTheme} = useTheme()

// 应用级 WebSocket 常驻连接（引用计数共享，App 不卸载故永不断连），
// 确保试验数据查看等非 Monitor 页面也能实时收到 port_data 事件
useBackendPorts()

/** 组件挂载后初始化认证状态，并自动启动测控服务（对齐原版"启动即初始化"） */
onMounted(() => {
  authStore.initAuth()
  // fj200cMainApi.startService().catch(() => {
  //   // 服务已在运行 / 后端未启动时静默忽略，不影响使用
  // })
})

/** 切换 CSV 数据录制状态 */
const onToggleRecording = async () => {
  try {
    const response = await fj200cMainApi.toggleRecording()
    if (response.success && response.data) {
      dashboardStore.isRecording = response.data.recording
    } else {
      ElMessage.error(response.message || '操作失败')
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}

/** 切换模拟运行状态（带确认框） */
const onToggleSimulation = async () => {
  const action = dashboardStore.isSimulating ? '停止' : '启动'
  try {
    await ElMessageBox.confirm(`确定要${action}模拟运行吗？`, '提示', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    })
  } catch {
    return // 用户取消
  }
  try {
    const response = await fj200cMainApi.toggleSimulation()
    if (response.success && response.data) {
      dashboardStore.isSimulating = response.data.simulating
    } else {
      ElMessage.error(response.message || '操作失败')
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}
</script>

<template>
  <el-config-provider :locale="zhCn">
    <div id="app">
      <!-- 全局导航条（登录页除外），#actions 插槽放三个操作按钮 -->
      <AppNavbar v-if="!isLoginPage">
        <template #actions>
          <el-button
              :type="dashboardStore.isRecording ? 'danger' : 'primary'"
              size="small"
              @click="onToggleRecording"
          >
            {{ dashboardStore.isRecording ? '停止保存' : '保存数据' }}
          </el-button>
          <el-button
              :type="dashboardStore.isSimulating ? 'warning' : 'success'"
              size="small"
              @click="onToggleSimulation"
          >
            {{ dashboardStore.isSimulating ? '停止模拟' : '模拟运行' }}
          </el-button>
          <el-button size="small" @click="toggleTheme">
            {{ isDark ? '浅色主题' : '深色主题' }}
          </el-button>
        </template>
      </AppNavbar>
      <router-view/>
    </div>
  </el-config-provider>
</template>

<style>
/* 全局样式（非 scoped），作用于整个应用 */
#app {
  font-family: 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', '微软雅黑', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  min-height: 100vh;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
</style>
