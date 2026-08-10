/**
 * 时钟组合式函数
 *
 * 每秒自动更新当前时间，提供 formatted() 方法返回格式化时间字符串。
 * 组件卸载时自动清除定时器，防止内存泄漏。
 *
 * 组合式函数命名约定：use 前缀 + 功能名（useClock）
 * 返回值约定：返回包含响应式数据和方法的对象
 */

import { onUnmounted, ref } from "vue";

/**
 * 创建每秒更新的时钟
 *
 * @returns 包含 now（当前 Date 对象）和 formatted（格式化函数）的对象
 */
export function useClock() {
  /** 当前时间（每秒更新触发响应式） */
  const now = ref(new Date());

  /** 每秒更新一次时间 */
  const timer = setInterval(() => {
    now.value = new Date();
  }, 1000);

  /**
   * 组件卸载时清除定时器
   * onUnmounted 是 Vue 3 生命周期钩子，组件销毁后执行
   */
  onUnmounted(() => clearInterval(timer));

  /**
   * 格式化时间为 "YYYY-MM-DD HH:mm:ss" 格式
   *
   * padStart(2, "0")：将数字补零到 2 位（如 9 → "09"）
   */
  const formatted = () => {
    const d = now.value;
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  };

  return { now, formatted };
}
