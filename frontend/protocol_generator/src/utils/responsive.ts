/**
 * 响应式工具模块（re-export @shared）
 *
 * 实现已统一收敛到 packages/shared/src/responsive.ts
 * （@shared 已可直接提供组件/组合式函数，vue-tsc 2.x 验证通过），
 * 本文件仅作 re-export，保持既有调用点不变。
 */
export { breakpoints, useResponsive, useLayoutConfig } from "@shared/responsive";
