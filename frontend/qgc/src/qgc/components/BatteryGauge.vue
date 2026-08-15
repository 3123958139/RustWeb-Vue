<!--
  电池电量计（BatteryGauge，HUD 风格）

  SVG 电池图标（5 格电量）+ 大字百分比 + 电压/电流读数 + 电量进度条。
  表现力升级：
  1. 电池分格渐变点亮（发光效果），低于 20% 切换红色并脉冲告警
  2. 电压 / 电流双读数
  3. 底部细进度条（发光渐变）
-->
<template>
  <div class="battery-gauge" :class="{ low: remaining < 20 }">
    <div class="battery-icon">
      <svg viewBox="0 0 120 58" class="battery-svg">
        <defs>
          <linearGradient id="qgc-batt-grad" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stop-color="#00d4aa" />
            <stop offset="100%" stop-color="#7dffc4" />
          </linearGradient>
          <linearGradient id="qgc-batt-grad-low" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stop-color="#c0392b" />
            <stop offset="100%" stop-color="#ff6a5e" />
          </linearGradient>
        </defs>
        <!-- 电池外壳 + 正极凸点 -->
        <rect x="2" y="10" width="106" height="38" rx="7" class="batt-shell" />
        <rect x="110" y="21" width="8" height="16" rx="2" class="batt-nub" />
        <!-- 5 格电量：第 i 格在电量 >= i*20% 时点亮 -->
        <g v-for="i in 5" :key="i">
          <rect :x="7 + (i - 1) * 20" y="15" width="17" height="28" rx="3" class="batt-cell" :class="{ on: i * 20 <= remaining }" />
        </g>
      </svg>
      <div class="batt-pct">
        <span class="pct-value">{{ remaining.toFixed(0) }}</span>
        <span class="pct-unit">%</span>
      </div>
    </div>

    <div class="batt-info">
      <div class="batt-row">
        <span class="batt-label">电压</span>
        <span class="batt-value">{{ voltage.toFixed(1) }} V</span>
      </div>
      <div class="batt-row">
        <span class="batt-label">电流</span>
        <span class="batt-value">{{ current.toFixed(1) }} A</span>
      </div>
      <div class="batt-row">
        <span class="batt-label">已耗电量</span>
        <span class="batt-value">{{ consumedMah.toFixed(0) }} mAh</span>
      </div>
      <div class="batt-bar">
        <div class="batt-fill" :style="{ width: heightPct + '%' }"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    remaining: number;
    voltage: number;
    /** 电池电流（A） */
    current: number;
    /** 已消耗电量（mAh） */
    consumedMah: number;
  }>(),
  { current: 0, consumedMah: 0 },
);

/** 电量百分比（clamp 0~100） */
const heightPct = computed(() => Math.max(0, Math.min(100, props.remaining)));
</script>

<style scoped>
.battery-gauge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 4px 0;
}

.battery-icon {
  position: relative;
  width: 100%;
  max-width: 240px;
}

.battery-svg {
  width: 100%;
  display: block;
  filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.4));
}

.batt-shell {
  fill: #0a1428;
  stroke: rgba(0, 180, 216, 0.35);
  stroke-width: 1.5;
}

.batt-nub {
  fill: #0a1428;
  stroke: rgba(0, 180, 216, 0.35);
  stroke-width: 1.5;
}

.batt-cell {
  fill: #16233a;
  transition: fill 0.3s ease;
}

/* 点亮格：渐变 + 发光 */
.batt-cell.on {
  fill: url(#qgc-batt-grad);
  filter: drop-shadow(0 0 4px rgba(0, 212, 170, 0.7));
}

/* 低电量：红色渐变 + 脉冲告警 */
.battery-gauge.low .batt-cell.on {
  fill: url(#qgc-batt-grad-low);
  filter: drop-shadow(0 0 5px rgba(255, 51, 85, 0.8));
  animation: batt-blink 1s ease-in-out infinite;
}

.batt-pct {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  align-items: baseline;
  gap: 3px;
  pointer-events: none;
}

.pct-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 36px;
  font-weight: 700;
  color: #8affc4;
  text-shadow: 0 0 10px rgba(0, 212, 170, 0.7);
}

.pct-unit {
  font-size: 15px;
  color: #9fb0c0;
}

.battery-gauge.low .pct-value {
  color: #ff6a5e;
  text-shadow: 0 0 10px rgba(255, 51, 85, 0.8);
}

.batt-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  max-width: 240px;
}

.batt-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}

.batt-label {
  font-size: 12px;
  color: #7d94b5;
}

.batt-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 16px;
  color: #d6e6f5;
}

.batt-bar {
  height: 7px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  overflow: hidden;
}

.batt-fill {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, #00d4aa, #7dffc4);
  box-shadow: 0 0 8px rgba(0, 212, 170, 0.6);
  transition: width 0.3s ease;
}

.battery-gauge.low .batt-fill {
  background: linear-gradient(90deg, #c0392b, #ff6a5e);
  box-shadow: 0 0 8px rgba(255, 51, 85, 0.7);
}

@keyframes batt-blink {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.45;
  }
}
</style>
