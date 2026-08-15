<!--
  电池电量计（BatteryGauge）

  垂直刻度条 + 百分比数值；低电量（< 20%）变红。
-->
<template>
  <div class="battery-gauge">
    <div class="gauge-bar">
      <div class="gauge-fill" :class="{ low: remaining < 20 }" :style="{ height: heightPct + '%' }"></div>
    </div>
    <div class="gauge-info">
      <span class="pct" :class="{ low: remaining < 20 }">{{ remaining.toFixed(0) }}%</span>
      <span class="voltage">{{ voltage.toFixed(1) }}V</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  remaining: number;
  voltage: number;
}>();

/** 电量百分比（clamp 0~100） */
const heightPct = computed(() => Math.max(0, Math.min(100, props.remaining)));
</script>

<style scoped>
.battery-gauge {
  display: flex;
  align-items: center;
  gap: 10px;
}

.gauge-bar {
  position: relative;
  width: 22px;
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
  background: linear-gradient(180deg, #5cb85c, #3a7d3a);
  transition: height 0.3s ease;
}

.gauge-fill.low {
  background: linear-gradient(180deg, #d9534f, #8a2d2a);
}

.gauge-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.pct {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 20px;
  color: #ffffff;
}

.pct.low {
  color: #d9534f;
}

.voltage {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: #a0a0a0;
}
</style>
