//! # 主题切换（深色/浅色）
//!
//! 管理应用主题切换。在 `<html>` 标签上切换 `theme-dark` / `theme-light` CSS 类
//! （驱动仪表盘 CSS 变量），同时切换 `dark` 类以联动 Element Plus 暗色主题。
//!
//! - `applyTheme(dark)`：模块级函数，供 useBackendPorts 收到 WS `theme_state` 事件时调用
//! - `toggle()`：用户主动切换时调用，并通过 `fj200cMainApi.setTheme()` 同步到服务端

import { ref } from 'vue'
import { fj200cMainApi } from '@/api'
import { useDashboardStore } from '@/fj200c_main/store/dashboard'

const isDark = ref(true)

export function applyTheme(dark: boolean) {
  isDark.value = dark
  document.documentElement.classList.toggle('theme-dark', dark)
  document.documentElement.classList.toggle('theme-light', !dark)
  document.documentElement.classList.toggle('dark', dark)
  localStorage.setItem('theme', dark ? 'dark' : 'light')
  const store = useDashboardStore()
  store.isDark = dark
  window.dispatchEvent(new CustomEvent('theme-changed', { detail: { isDark: dark } }))
}

export function useTheme() {
  const saved = localStorage.getItem('theme')
  const initialDark = saved !== 'light'
  applyTheme(initialDark)

  function toggle() {
    const next = !isDark.value
    applyTheme(next)
    fj200cMainApi.setTheme(next).catch(() => {
      // 服务端同步失败仅记录，不影响本地主题
    })
  }

  return { isDark, toggle }
}
