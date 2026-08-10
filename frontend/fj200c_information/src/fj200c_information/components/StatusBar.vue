<!--
  底部状态栏组件（从 fj200c_information.informatization 的 StatusBar.vue 移植）

  Props：
  - statusText       — 状态描述文字（如"服务运行中..."）
  - connectionStatus — 连接状态（"已连接" / "未连接"）
  - currentTime      — 当前时间字符串（由父组件的 useClock 提供）

  Vue 3 类型化 Props：
  defineProps<{ ... }>() 是 <script setup> 中的编译器宏，
  无需 import，TypeScript 类型标注提供运行时类型检查。
-->

<script lang="ts" setup>
/**
 * defineProps：声明组件接收的 props
 * TypeScript 泛型写法提供完整的类型推导
 */
defineProps<{
  statusText: string;
  connectionStatus: string;
  currentTime: string;
}>();
</script>

<template>
  <div class="status-bar">
    <div class="status-bar-left">
      <!-- 动态 class：根据连接状态切换指示灯颜色 -->
      <span
        class="status-indicator"
        :class="connectionStatus === '已连接' ? 'connected' : 'disconnected'"
      ></span>
      <span class="status-item">状态: {{ statusText }}</span>
      <span class="status-divider">|</span>
      <span class="status-item">连接: {{ connectionStatus }}</span>
    </div>
    <div class="status-bar-right">
      <span class="status-item">{{ currentTime }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: #f0f0f0;
  border-top: 1px solid #d9d9d9;
  font-size: 13px;
  flex-shrink: 0;
}

.status-bar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-bar-right {
  display: flex;
  align-items: center;
}

.status-indicator {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-indicator.connected {
  background-color: #52c41a;
}

.status-indicator.disconnected {
  background-color: #ff4d4f;
}

.status-item {
  color: #333;
}

.status-divider {
  color: #ccc;
}
</style>
