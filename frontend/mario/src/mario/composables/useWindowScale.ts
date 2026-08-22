/**
 * @module useWindowScale
 * @description 游戏界面等比自适应缩放
 *
 * 参考 fj200c_main 的 `useWindowScale`：基于 `ResizeObserver` 监听容器尺寸，
 * 将设计稿尺寸（默认 480×320 的游戏画布）等比缩放到任意屏幕。
 *
 * 与 fj200c_main 的差异：游戏要求**等比**缩放（不拉伸变形），故取
 * `min(宽比, 高比)` 而非 x/y 分离；另用 `maxScale` 限制最大放大倍数，
 * 超出时居中留黑边（避免像素画被过度放大糊成马赛克）。
 */

import { ref, computed, onMounted, onUnmounted } from "vue";

/** 缩放配置项 */
export interface WindowScaleOptions {
  /** 设计稿宽度（默认 480） */
  designWidth?: number
  /** 设计稿高度（默认 320） */
  designHeight?: number
  /** 最大等比缩放比（null 表示不限制） */
  maxScale?: number | null
}

export function useWindowScale(options: WindowScaleOptions = {}) {
  const DESIGN_W = options.designWidth ?? 480;
  const DESIGN_H = options.designHeight ?? 320;
  const maxScale = options.maxScale ?? 6;

  const _s = ref(1);
  const rootRef = ref<HTMLDivElement>();
  let ro: ResizeObserver | null = null;
  let rafId: number | null = null;

  const scale = computed(() => _s.value);

  function updateScale() {
    if (!rootRef.value) return;
    const w = rootRef.value.clientWidth;
    const h = rootRef.value.clientHeight;
    if (w === 0 || h === 0) return;
    const s = Math.min(w / DESIGN_W, h / DESIGN_H);
    _s.value = maxScale == null ? s : Math.min(s, maxScale);
  }

  function throttledUpdate() {
    if (rafId !== null) return;
    rafId = requestAnimationFrame(() => {
      rafId = null;
      updateScale();
    });
  }

  onMounted(() => {
    updateScale();
    ro = new ResizeObserver(throttledUpdate);
    if (rootRef.value) ro.observe(rootRef.value);
  });

  onUnmounted(() => {
    ro?.disconnect();
    if (rafId !== null) cancelAnimationFrame(rafId);
  });

  return { scale, rootRef, DESIGN_W, DESIGN_H };
}