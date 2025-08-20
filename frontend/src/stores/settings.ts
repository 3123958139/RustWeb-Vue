import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService, type UserSettings, type UpdateSettingsRequest } from '@/api'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<UserSettings | null>(null)
  const loading = ref(false)
  const saving = ref(false)

  // 计算属性
  const isDarkMode = computed(() => settings.value?.theme === 'dark')
  const primaryColor = computed(() => settings.value?.primary_color || '#409eff')
  const language = computed(() => settings.value?.language || 'zh-CN')
  const timezone = computed(() => settings.value?.timezone || 'Asia/Shanghai')

  // 加载设置
  const loadSettings = async () => {
    loading.value = true
    try {
      const response = await apiService.getSettings()
      if (response.success && response.data) {
        settings.value = response.data
        applyThemeSettings()
      }
    } catch (error) {
      console.error('加载设置失败:', error)
      throw error
    } finally {
      loading.value = false
    }
  }

  // 更新设置
  const updateSettings = async (settingsData: UpdateSettingsRequest) => {
    saving.value = true
    try {
      const response = await apiService.updateSettings(settingsData)
      if (response.success && response.data) {
        settings.value = response.data
        applyThemeSettings()
        return response.data
      } else {
        throw new Error(response.message || '更新失败')
      }
    } catch (error) {
      console.error('更新设置失败:', error)
      throw error
    } finally {
      saving.value = false
    }
  }

  // 应用主题设置
  const applyThemeSettings = () => {
    if (!settings.value) return

    // 应用主题模式
    if (settings.value.theme === 'dark') {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }

    // 应用主题色彩
    if (settings.value.primary_color) {
      document.documentElement.style.setProperty('--el-color-primary', settings.value.primary_color)
    }
  }

  // 重置设置
  const resetSettings = async () => {
    const defaultSettings: UpdateSettingsRequest = {
      theme: 'light',
      primary_color: '#409eff',
      email_notifications: true,
      browser_notifications: false,
      notification_types: ['posts', 'system'],
      two_factor_auth: false,
      session_timeout: 60,
      profile_visibility: 'public',
      default_post_visibility: 'public',
      data_collection: true,
      language: 'zh-CN',
      timezone: 'Asia/Shanghai'
    }

    return await updateSettings(defaultSettings)
  }

  return {
    settings,
    loading,
    saving,
    isDarkMode,
    primaryColor,
    language,
    timezone,
    loadSettings,
    updateSettings,
    applyThemeSettings,
    resetSettings
  }
})
