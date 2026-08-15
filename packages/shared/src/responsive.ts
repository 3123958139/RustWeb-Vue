/**
 * 响应式工具模块（8 个用户端应用共享）
 *
 * 基于 VueUse 的 useWindowSize 和 useBreakpoints 封装，
 * 提供响应式断点检测和布局配置。
 *
 * 历史：该文件曾因 Volar 模板 ref 解包的旧限制在每个应用中各维护一份副本；
 * 当前 @shared 已直接提供 Vue 组件（LoginPage/AppNavbar 等）且 vue-tsc 2.x 通过，
 * 故统一收敛到 @shared，各应用 `src/utils/responsive.ts` 改为 re-export 保持调用点不变。
 *
 * 断点定义（参考 Tailwind CSS）：
 * - xs: < 480px    （手机竖屏）
 * - sm: 640px       （手机横屏）
 * - md: 768px       （平板竖屏）
 * - lg: 1024px      （平板横屏/小桌面）
 * - xl: 1280px      （桌面）
 * - 2xl: 1536px     （大桌面）
 */
import { useWindowSize, useBreakpoints, breakpointsTailwind } from "@vueuse/core";
import { computed } from "vue";

/** 自定义断点阈值（像素），与 Tailwind 默认值一致 */
export const breakpoints = {
  xs: 480,
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
};

/**
 * 响应式断点检测组合式函数
 *
 * 使用 Vue 3 Composition API 封装，返回的 computed 响应式引用
 * 会在窗口大小变化时自动更新。
 *
 * @returns 包含宽度、当前断点、设备类型判断等响应式数据
 */
export const useResponsive = () => {
  /** 获取当前窗口宽度（响应式） */
  const { width } = useWindowSize();
  /** VueUse 提供的 Tailwind 断点工具 */
  const breakpointsData = useBreakpoints(breakpointsTailwind);

  /** 当前断点名称（响应式计算属性） */
  const currentBreakpoint = computed(() => {
    if (width.value < breakpoints.xs) return "xs";
    if (width.value < breakpoints.sm) return "sm";
    if (width.value < breakpoints.md) return "md";
    if (width.value < breakpoints.lg) return "lg";
    if (width.value < breakpoints.xl) return "xl";
    return "2xl";
  });

  /** 是否为移动设备（< 768px） */
  const isMobile = computed(() => width.value < breakpoints.md);

  /** 是否为平板（768px ~ 1023px） */
  const isTablet = computed(
    () => width.value >= breakpoints.md && width.value < breakpoints.lg
  );

  /** 是否为桌面（>= 1024px） */
  const isDesktop = computed(() => width.value >= breakpoints.lg);

  /** 是否为小屏幕（< 640px） */
  const isSmallScreen = computed(() => width.value < breakpoints.sm);

  /** 是否为中等屏幕（768px ~ 1279px） */
  const isMediumScreen = computed(
    () => width.value >= breakpoints.md && width.value < breakpoints.xl
  );

  /** 是否为大屏幕（>= 1280px） */
  const isLargeScreen = computed(() => width.value >= breakpoints.xl);

  return {
    width,
    currentBreakpoint,
    isMobile,
    isTablet,
    isDesktop,
    isSmallScreen,
    isMediumScreen,
    isLargeScreen,
    breakpointsData,
  };
};

/**
 * 响应式布局配置组合式函数
 *
 * 根据当前设备类型（移动/平板/桌面）返回不同的布局参数，
 * 用于在模板中动态绑定样式或传递给 Element Plus 组件。
 *
 * @returns layoutConfig —— 包含 sidebar / header / content / card / form / table 配置的计算属性
 */
export const useLayoutConfig = () => {
  const { isMobile, isTablet, isDesktop } = useResponsive();

  /** 响应式布局配置（计算属性，设备类型变化时自动更新） */
  const layoutConfig = computed(() => ({
    // 侧边栏配置
    sidebar: {
      width: isMobile.value ? "100%" : isTablet.value ? "280px" : "320px",
      collapsed: isMobile.value,         // 移动端默认折叠
      showOverlay: isMobile.value,       // 移动端使用遮罩层覆盖
    },

    // 头部配置
    header: {
      height: isMobile.value ? "56px" : "64px",
      showLogo: !isMobile.value,         // 移动端隐藏 logo 节省空间
      showMenu: isMobile.value,          // 移动端显示汉堡菜单
    },

    // 内容区域配置
    content: {
      padding: isMobile.value ? "16px" : isTablet.value ? "24px" : "32px",
      maxWidth: isDesktop.value ? "1200px" : "100%",
    },

    // 卡片配置
    card: {
      padding: isMobile.value ? "16px" : "24px",
      margin: isMobile.value ? "8px 0" : "16px 0",
    },

    // 表单配置
    form: {
      labelWidth: isMobile.value ? "60px" : "80px",
      labelPosition: isMobile.value ? "top" : "left", // 移动端标签在上方
    },

    // 表格配置
    table: {
      stripe: !isMobile.value,           // 移动端取消斑马纹
      border: isMobile.value,            // 移动端加边框提高可读性
      size: isMobile.value ? "small" : "default",
    },
  }));

  return {
    layoutConfig,
  };
};
