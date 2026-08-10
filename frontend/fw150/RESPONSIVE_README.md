# 响应式设计说明

## 概述

本项目已经实现了完整的响应式设计，支持手机、平板和桌面端的自适应布局。

## 技术栈

### 核心库

- **VueUse** (`@vueuse/core`) - 提供响应式工具函数
- **Element Plus** - 内置响应式组件
- **CSS Grid & Flexbox** - 现代 CSS 布局技术

### 响应式工具

#### 1. 断点检测 (`src/utils/responsive.ts`)

```typescript
// 使用响应式工具
const { isMobile, isTablet, isDesktop, currentBreakpoint } = useResponsive();

// 断点定义
const breakpoints = {
  xs: 480, // 超小屏幕
  sm: 640, // 小屏幕
  md: 768, // 中等屏幕
  lg: 1024, // 大屏幕
  xl: 1280, // 超大屏幕
  "2xl": 1536, // 超超大屏幕
};
```

#### 2. 布局配置 (`src/utils/responsive.ts`)

```typescript
// 使用布局配置
const { layoutConfig } = useLayoutConfig();

// 自动适配的配置
const config = {
  sidebar: { width: "320px", collapsed: false },
  header: { height: "64px", showLogo: true },
  content: { padding: "32px", maxWidth: "1200px" },
  card: { padding: "24px", margin: "16px 0" },
  form: { labelWidth: "80px", labelPosition: "left" },
  table: { stripe: true, border: false, size: "default" },
};
```

## 响应式特性

### 1. 移动端优化 (< 768px)

- **字体大小**: 14px (基础字体)
- **间距**: 8px-16px
- **布局**: 单列布局
- **交互**: 触摸优化
- **隐藏元素**: 桌面端专用元素

### 2. 平板端优化 (768px - 1023px)

- **布局**: 双列布局
- **间距**: 16px-24px
- **字体**: 16px (基础字体)

### 3. 桌面端优化 (≥ 1024px)

- **布局**: 多列布局
- **间距**: 24px-32px
- **字体**: 16px (基础字体)
- **显示元素**: 完整功能

## 页面响应式实现

### 1. 登录/注册页面

- **移动端**: 全宽卡片，标签在上方
- **桌面端**: 居中卡片，标签在左侧
- **按钮**: 移动端全宽，桌面端自适应

### 2. 仪表板页面

- **统计卡片**: 移动端单列，桌面端四列
- **快速操作**: 移动端双列，桌面端四列
- **表格**: 移动端隐藏部分列，桌面端完整显示

### 3. 文章列表页面

- **视图切换**: 表格视图和卡片视图
- **筛选器**: 移动端垂直排列，桌面端水平排列
- **分页**: 响应式分页组件

## CSS 工具类

### 响应式断点

```css
/* 移动端 */
@media (max-width: 767px) {
  ...;
}

/* 平板端 */
@media (min-width: 768px) and (max-width: 1023px) {
  ...;
}

/* 桌面端 */
@media (min-width: 1024px) {
  ...;
}
```

### 工具类

```css
/* 显示/隐藏 */
.desktop-only {
  display: none;
} /* 移动端隐藏 */
.mobile-only {
  display: none;
} /* 桌面端隐藏 */

/* 布局 */
.container {
  max-width: 1200px;
  margin: 0 auto;
}
.grid {
  display: grid;
  gap: 16px;
}

/* 响应式网格 */
.grid-cols-1 {
  grid-template-columns: repeat(1, 1fr);
}
.sm\:grid-cols-2 {
  grid-template-columns: repeat(2, 1fr);
}
.md\:grid-cols-3 {
  grid-template-columns: repeat(3, 1fr);
}
.lg\:grid-cols-4 {
  grid-template-columns: repeat(4, 1fr);
}
```

## 使用指南

### 1. 在组件中使用响应式工具

```vue
<script setup>
import { useResponsive, useLayoutConfig } from "@/utils/responsive";

const { isMobile, isTablet, isDesktop } = useResponsive();
const { layoutConfig } = useLayoutConfig();
</script>

<template>
  <div :class="{ 'mobile-layout': isMobile, 'desktop-layout': isDesktop }">
    <el-card :body-style="{ padding: layoutConfig.card.padding }">
      <!-- 内容 -->
    </el-card>
  </div>
</template>
```

### 2. 条件渲染

```vue
<template>
  <!-- 移动端显示 -->
  <div v-if="isMobile" class="mobile-menu">
    <!-- 移动端菜单 -->
  </div>

  <!-- 桌面端显示 -->
  <div v-if="isDesktop" class="desktop-menu">
    <!-- 桌面端菜单 -->
  </div>
</template>
```

### 3. 动态样式

```vue
<template>
  <div
    :style="{
      fontSize: isMobile ? '14px' : '16px',
      padding: isMobile ? '16px' : '32px',
    }"
  >
    <!-- 内容 -->
  </div>
</template>
```

## 最佳实践

### 1. 移动优先设计

- 先设计移动端布局
- 逐步增强到桌面端
- 使用渐进式增强

### 2. 性能优化

- 使用 CSS Grid 和 Flexbox
- 避免 JavaScript 计算布局
- 合理使用媒体查询

### 3. 用户体验

- 触摸友好的按钮大小 (≥ 44px)
- 合适的字体大小 (≥ 14px)
- 清晰的视觉层次

### 4. 测试

- 在不同设备上测试
- 使用浏览器开发者工具
- 测试横屏和竖屏模式

## 浏览器支持

- **现代浏览器**: Chrome, Firefox, Safari, Edge
- **移动浏览器**: iOS Safari, Chrome Mobile
- **最低版本**:
  - CSS Grid: IE 11 (部分支持)
  - Flexbox: IE 10+
  - Vue 3: 现代浏览器

## 扩展建议

### 1. 添加更多断点

```typescript
const customBreakpoints = {
  "3xl": 1920,
  "4xl": 2560,
};
```

### 2. 主题适配

```typescript
// 深色模式支持
const isDarkMode = useDark();
const theme = computed(() => (isDarkMode.value ? "dark" : "light"));
```

### 3. 性能监控

```typescript
// 监控布局变化
const { width, height } = useWindowSize();
watch([width, height], () => {
  // 处理布局变化
});
```

## 总结

本项目的响应式设计提供了：

- ✅ 完整的移动端适配
- ✅ 流畅的桌面端体验
- ✅ 灵活的布局系统
- ✅ 优秀的用户体验
- ✅ 易于维护的代码结构

通过使用现代 CSS 技术和 VueUse 工具库，我们实现了高效、可维护的响应式设计系统。
