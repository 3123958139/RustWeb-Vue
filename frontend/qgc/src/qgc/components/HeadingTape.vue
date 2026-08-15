<!--
  航向刻度带（HeadingTape）

  横向刻度带，中心指针指示当前航向，刻度从 -90 到 +90（相对当前航向）。
-->
<template>
  <div class="heading-tape">
    <div class="tape-window">
      <div class="tape" :style="{ transform: `translateX(${translateX}px)` }">
        <!-- 相对刻度：每 10 度一格，-90 ~ +90 -->
        <div v-for="rel in relTicks" :key="rel" class="tick" :style="{ left: (rel + 90) * 2 + 'px' }">
          <span v-if="rel % 30 === 0" class="tick-label">{{ normalize((heading + rel + 360) % 360).toFixed(0).padStart(3, "0") }}</span>
          <span class="tick-mark" :class="{ major: rel % 30 === 0 }"></span>
        </div>
      </div>
      <!-- 中心指针 -->
      <div class="center-line"></div>
    </div>
    <div class="heading-value">{{ normalize(heading).toFixed(1) }}°</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ heading: number }>();

/** 刻度带中心偏移：heading 越大，刻度带向左移动越多（1 度 = 2px） */
const translateX = computed(() => -props.heading * 2);
/** 相对刻度数组：-90 ~ +90 */
const relTicks = computed(() => {
  const ticks: number[] = [];
  for (let rel = -90; rel <= 90; rel += 10) ticks.push(rel);
  return ticks;
});

/** 归一化到 0~360 */
function normalize(h: number): number {
  return ((h % 360) + 360) % 360;
}
</script>

<style scoped>
.heading-tape {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.tape-window {
  position: relative;
  flex: 1;
  height: 56px;
  overflow: hidden;
  background: #141518;
  border: 1px solid #2c2f36;
  border-radius: 6px;
}

.tape {
  position: absolute;
  top: 0;
  left: 50%;
  height: 100%;
  width: 360px;
  margin-left: -180px;
  transition: transform 0.2s linear;
}

.tick {
  position: absolute;
  top: 0;
  height: 100%;
}

.tick-mark {
  position: absolute;
  top: 30px;
  left: -1px;
  width: 2px;
  height: 10px;
  background: #e0e0e0;
}

.tick-mark.major {
  height: 16px;
  background: #ffffff;
}

.tick-label {
  position: absolute;
  top: 4px;
  left: 0;
  transform: translateX(-50%);
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: #a0a0a0;
  white-space: nowrap;
}

.center-line {
  position: absolute;
  top: 0;
  left: 50%;
  width: 2px;
  height: 100%;
  background: #ffcc00;
}

.heading-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 20px;
  color: #ffffff;
  min-width: 90px;
  text-align: right;
}
</style>
