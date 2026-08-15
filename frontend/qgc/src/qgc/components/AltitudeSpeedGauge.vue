<!--
  高度/速度表（AltitudeSpeedGauge）

  双列垂直刻度：左侧高度（米）、右侧速度（m/s），中央数值大字显示。
-->
<template>
  <div class="gauge-row">
    <div class="gauge-column">
      <div class="gauge-bar">
        <div class="gauge-fill" :style="{ height: altPct + '%' }"></div>
      </div>
      <div class="gauge-value">
        <span class="label">高度</span>
        <span class="num">{{ relativeAlt.toFixed(1) }} m</span>
      </div>
    </div>
    <div class="gauge-column">
      <div class="gauge-bar">
        <div class="gauge-fill" :style="{ height: spdPct + '%' }"></div>
      </div>
      <div class="gauge-value">
        <span class="label">速度</span>
        <span class="num">{{ groundspeed.toFixed(1) }} m/s</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  relativeAlt: number;
  groundspeed: number;
  climb: number;
}>();

/** 高度百分比（0~100 米映射 0~100%，超限 clamp） */
const altPct = computed(() => Math.max(0, Math.min(100, (props.relativeAlt / 100) * 100)));
/** 速度百分比（0~50 m/s 映射 0~100%，超限 clamp） */
const spdPct = computed(() => Math.max(0, Math.min(100, (props.groundspeed / 50) * 100)));
</script>

<style scoped>
.gauge-row {
  display: flex;
  gap: 24px;
  align-items: center;
}

.gauge-column {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.gauge-bar {
  position: relative;
  width: 18px;
  height: 110px;
  background: #141518;
  border: 1px solid #2c2f36;
  border-radius: 4px;
  overflow: hidden;
}

.gauge-fill {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: linear-gradient(180deg, #4aa3ff, #2a6bbd);
  transition: height 0.3s ease;
}

.gauge-value {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.label {
  font-size: 11px;
  color: #a0a0a0;
}

.num {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 16px;
  color: #ffffff;
  white-space: nowrap;
}
</style>
