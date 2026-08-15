<!--
  航向刻度带（HeadingTape，HUD 罗盘）

  横向刻度带，中心指针指示当前航向，刻度从 -90 到 +90（相对当前航向）。
  表现力升级：
  1. 每 30° 主刻度显示罗盘方位字（N/E/S/W 高亮青色，其余为数字）
  2. 主刻度发光青色、次刻度灰白
  3. 中心发光三角形指针 + 左右渐变遮罩
  4. 右侧大字读数（HDG + 数值，发光）
-->
<template>
  <div class="heading-tape">
    <div class="tape-window">
      <div class="tape" :style="{ transform: `translateX(${translateX}px)` }">
        <!-- 相对刻度：每 10 度一格，-90 ~ +90 -->
        <div v-for="rel in relTicks" :key="rel" class="tick" :style="{ left: (rel + 90) * 2 + 'px' }">
          <span v-if="rel % 30 === 0" class="tick-label" :class="{ cardinal: tickInfo(rel).cardinal }">{{ tickInfo(rel).text }}</span>
          <span class="tick-mark" :class="{ major: rel % 30 === 0 }"></span>
        </div>
      </div>
      <!-- 中心发光指针 -->
      <div class="pointer"><div class="pointer-tip"></div></div>
    </div>
    <div class="heading-readout">
      <span class="readout-label">HDG</span>
      <span class="readout-value">{{ normalize(heading).toFixed(1) }}°</span>
    </div>
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

/** 罗盘方位字（0/90/180/270 显示 N/E/S/W） */
const cardinalMap: Record<number, string> = { 0: "N", 90: "E", 180: "S", 270: "W" };

/**
 * 主刻度文字：方位角四字用字母，其余用三位数字
 *
 * @param rel - 相对当前航向的刻度偏移
 * @returns 显示文字与是否方位字
 */
function tickInfo(rel: number) {
  const h = Math.round(normalize(props.heading + rel) / 30) * 30;
  const cardinal = h % 90 === 0;
  return { text: cardinal ? cardinalMap[h] : String(h).padStart(3, "0"), cardinal };
}
</script>

<style scoped>
.heading-tape {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.tape-window {
  position: relative;
  flex: 1;
  height: 108px;
  overflow: hidden;
  background: linear-gradient(180deg, #0a1428 0%, #0f1d33 100%);
  border: 1px solid var(--border-color, #1e3a5f);
  border-radius: 10px;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05), 0 4px 12px rgba(0, 0, 0, 0.35);
}

/* 左右渐变遮罩 */
.tape-window::before,
.tape-window::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  width: 34px;
  z-index: 1;
  pointer-events: none;
}

.tape-window::before {
  left: 0;
  background: linear-gradient(90deg, #0a1428, transparent);
}

.tape-window::after {
  right: 0;
  background: linear-gradient(270deg, #0a1428, transparent);
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
  top: 62px;
  left: -1px;
  width: 2px;
  height: 12px;
  background: rgba(180, 196, 210, 0.5);
}

.tick-mark.major {
  height: 22px;
  background: var(--text-accent, #00b4d8);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.8);
}

.tick-label {
  position: absolute;
  top: 8px;
  left: 0;
  transform: translateX(-50%);
  font-family: "Consolas", "Courier New", monospace;
  font-size: 14px;
  color: #9fb0c0;
  white-space: nowrap;
}

.tick-label.cardinal {
  color: var(--text-accent, #00b4d8);
  font-weight: 700;
  text-shadow: 0 0 6px rgba(0, 180, 216, 0.7);
}

/* 中心指针：发光三角形 */
.pointer {
  position: absolute;
  top: -2px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 2;
}

.pointer-tip {
  width: 0;
  height: 0;
  border-left: 7px solid transparent;
  border-right: 7px solid transparent;
  border-top: 10px solid var(--text-accent, #00b4d8);
  filter: drop-shadow(0 0 4px rgba(0, 180, 216, 0.9));
}

.heading-readout {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  min-width: 130px;
}

.readout-label {
  font-size: 11px;
  letter-spacing: 3px;
  color: var(--text-dim, #7d94b5);
}

.readout-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 30px;
  font-weight: 700;
  color: #ffffff;
  text-shadow: 0 0 8px rgba(0, 180, 216, 0.6);
}
</style>
