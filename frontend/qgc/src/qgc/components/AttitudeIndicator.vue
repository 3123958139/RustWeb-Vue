<!--
  姿态仪（SVG 人工地平线）

  根据滚转角（roll）旋转地平线，根据俯仰角（pitch）平移地平线。
  模拟经典 QGC 姿态球外观：上方天空色、下方地面色、中央机身十字标记。
-->
<template>
  <div class="attitude-indicator">
    <svg viewBox="0 0 200 200" class="attitude-svg">
      <!-- 地平线组：随 roll 旋转、随 pitch 平移 -->
      <g :transform="`rotate(${roll} 100 100) translate(0 ${pitchOffset})`">
        <!-- 天空 -->
        <rect x="0" y="-100" width="200" height="200" fill="#3a7bd5" />
        <!-- 地面 -->
        <rect x="0" y="100" width="200" height="200" fill="#6b4a2b" />
        <!-- 地平线 -->
        <line x1="0" y1="100" x2="200" y2="100" stroke="#ffffff" stroke-width="2" />
        <!-- 俯仰刻度线（-30/-20/-10/10/20/30 度） -->
        <line
          v-for="deg in [-30, -20, -10, 10, 20, 30]"
          :key="deg"
          :x1="60"
          :y1="100 + deg * 2"
          :x2="140"
          :y2="100 + deg * 2"
          stroke="#ffffff"
          stroke-width="1"
          opacity="0.6"
        />
        <line v-for="deg in [-30, -20, -10, 10, 20, 30]" :key="'v' + deg" x1="100" y1="100" :x2="100 + deg * 3" y2="100 + deg * 2" stroke="#ffffff" stroke-width="1" opacity="0.8" />
      </g>
      <!-- 固定机身标记 -->
      <path d="M 92 100 L 108 100 L 102 108 L 98 108 Z" fill="#ffcc00" />
      <circle cx="100" cy="100" r="92" fill="none" stroke="#2c2f36" stroke-width="3" />
    </svg>
    <div class="attitude-values">
      <span class="value">{{ roll.toFixed(1) }}°</span>
      <span class="value">{{ pitch.toFixed(1) }}°</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  roll: number;
  pitch: number;
}>();

/** 俯仰角转换为像素平移（1 度 = 2px） */
const pitchOffset = computed(() => props.pitch * 2);
</script>

<style scoped>
.attitude-indicator {
  position: relative;
  width: 100%;
  max-width: 220px;
}

.attitude-svg {
  width: 100%;
  border-radius: 8px;
  display: block;
}

.attitude-values {
  position: absolute;
  bottom: 8px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-between;
  padding: 0 12px;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 13px;
  color: #e0e0e0;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
}

.value {
  background: rgba(0, 0, 0, 0.5);
  border-radius: 3px;
  padding: 1px 6px;
}
</style>
