/**
 * @module useWindowScale
 * @description 窗口自适应缩放（从 fj200c_main 移植）
 *
 * 基于 `ResizeObserver` 监听容器尺寸变化，动态计算 CSS scale 变换比例，
 * 将 1920×1080 设计稿适配到任意分辨率的窗口。
 *
 * 关键点：
 * - `ResizeObserver` 监听特定元素（比 window.resize 更精确）
 * - `transform: scale(x, y)` 以容器中心为原点缩放
 * - `screen-resize` 自定义事件通知图表等需要随尺寸重排的组件
 */

import { ref, computed, onMounted, onUnmounted } from 'vue'

/** 窗口缩放配置项 */
export interface WindowScaleOptions {
  /** 设计稿宽度（默认 1920） */
  designWidth?: number
  /** 设计稿高度（默认 1080） */
  designHeight?: number
  /** 最大缩放比（null 表示不限制） */
  maxScale?: number | null
}

export function useWindowScale(options: WindowScaleOptions = {}) {
  const DESIGN_W = options.designWidth ?? 1920
  const DESIGN_H = options.designHeight ?? 1080
  const maxScale = options.maxScale ?? null
  const _raw = ref({ x: 1, y: 1 })
  const rootRef = ref<HTMLDivElement>()
  let ro: ResizeObserver | null = null
  let rafId: number | null = null

  const scale = computed(() => {
    const r = _raw.value
    if (maxScale != null) {
      const u = Math.min(r.x, r.y, maxScale)
      return { x: u, y: u }
    }
    return r
  })

  const noScale = computed(() => maxScale != null && scale.value.x >= 1)

  function updateScale() {
    if (!rootRef.value) return
    const w = rootRef.value.clientWidth
    const h = rootRef.value.clientHeight
    if (w === 0 || h === 0) return
    _raw.value = { x: w / DESIGN_W, y: h / DESIGN_H }
    window.dispatchEvent(new CustomEvent('screen-resize', { detail: scale.value }))
  }

  function throttledUpdate() {
    if (rafId !== null) return
    rafId = requestAnimationFrame(() => {
      rafId = null
      updateScale()
    })
  }

  onMounted(() => {
    updateScale()
    ro = new ResizeObserver(throttledUpdate)
    if (rootRef.value) ro.observe(rootRef.value)
  })

  onUnmounted(() => {
    ro?.disconnect()
    if (rafId !== null) cancelAnimationFrame(rafId)
  })

  return { scale, rootRef, DESIGN_W, DESIGN_H, noScale }
}
