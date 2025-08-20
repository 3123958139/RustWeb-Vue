import { useWindowSize, useBreakpoints, breakpointsTailwind } from '@vueuse/core'
import { computed } from 'vue'

// 定义断点
export const breakpoints = {
  xs: 480,
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  '2xl': 1536,
}

// 使用 Tailwind 断点
export const useResponsive = () => {
  const { width } = useWindowSize()
  const breakpointsData = useBreakpoints(breakpointsTailwind)
  
  // 当前断点
  const currentBreakpoint = computed(() => {
    if (width.value < breakpoints.xs) return 'xs'
    if (width.value < breakpoints.sm) return 'sm'
    if (width.value < breakpoints.md) return 'md'
    if (width.value < breakpoints.lg) return 'lg'
    if (width.value < breakpoints.xl) return 'xl'
    return '2xl'
  })

  // 是否为移动设备
  const isMobile = computed(() => width.value < breakpoints.md)
  
  // 是否为平板
  const isTablet = computed(() => 
    width.value >= breakpoints.md && width.value < breakpoints.lg
  )
  
  // 是否为桌面
  const isDesktop = computed(() => width.value >= breakpoints.lg)
  
  // 是否为小屏幕
  const isSmallScreen = computed(() => width.value < breakpoints.sm)
  
  // 是否为中等屏幕
  const isMediumScreen = computed(() => 
    width.value >= breakpoints.md && width.value < breakpoints.xl
  )
  
  // 是否为大屏幕
  const isLargeScreen = computed(() => width.value >= breakpoints.xl)

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
  }
}

// 响应式布局配置
export const useLayoutConfig = () => {
  const { isMobile, isTablet, isDesktop } = useResponsive()
  
  const layoutConfig = computed(() => ({
    // 侧边栏配置
    sidebar: {
      width: isMobile.value ? '100%' : isTablet.value ? '280px' : '320px',
      collapsed: isMobile.value,
      showOverlay: isMobile.value,
    },
    
    // 头部配置
    header: {
      height: isMobile.value ? '56px' : '64px',
      showLogo: !isMobile.value,
      showMenu: isMobile.value,
    },
    
    // 内容区域配置
    content: {
      padding: isMobile.value ? '16px' : isTablet.value ? '24px' : '32px',
      maxWidth: isDesktop.value ? '1200px' : '100%',
    },
    
    // 卡片配置
    card: {
      padding: isMobile.value ? '16px' : '24px',
      margin: isMobile.value ? '8px 0' : '16px 0',
    },
    
    // 表单配置
    form: {
      labelWidth: isMobile.value ? '60px' : '80px',
      labelPosition: isMobile.value ? 'top' : 'left',
    },
    
    // 表格配置
    table: {
      stripe: !isMobile.value,
      border: isMobile.value,
      size: isMobile.value ? 'small' : 'default',
    },
  }))
  
  return {
    layoutConfig,
  }
}
