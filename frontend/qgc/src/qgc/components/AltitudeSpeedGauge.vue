<!--
  高度/速度表（AltitudeSpeedGauge，HUD 弧线仪表）

  双弧线仪表：左侧高度（0~120 m，青色）、右侧速度（0~60 m/s，绿色），
  中央数字大字读数 + 底部爬升率指示（▲上升/▼下降）。
  表现力升级：
  1. 240° 弧线刻度盘（背景弧 + 渐变发光值弧 + 刻度线）
  2. 端点量程标签（0 与满量程）
  3. 发光数字读数、爬升箭头颜色随方向变化
-->
<template>
  <div class="gauge-row">
    <!-- 高度弧线表 -->
    <div class="arc-gauge">
      <svg viewBox="0 0 140 150" class="arc-svg">
        <defs>
          <linearGradient id="qgc-alt-grad" x1="0" y1="1" x2="0" y2="0">
            <stop offset="0%" stop-color="#0077b6" />
            <stop offset="100%" stop-color="#00d4ff" />
          </linearGradient>
          <linearGradient id="qgc-spd-grad" x1="0" y1="1" x2="0" y2="0">
            <stop offset="0%" stop-color="#00d4aa" />
            <stop offset="100%" stop-color="#7dffc4" />
          </linearGradient>
        </defs>
        <path :d="altBgPath" class="arc-bg" />
        <path v-if="altFrac > 0" :d="altArcPath" class="arc-fg alt-fg" />
        <g v-for="t in altTicks" :key="'a' + t.v">
          <line :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2" class="arc-tick" :class="{ major: t.major }" />
          <text v-if="t.v === 0 || t.v === ALT_MAX" :x="t.lx" :y="t.ly" class="arc-label" text-anchor="middle">{{ t.v }}</text>
        </g>
      </svg>
      <div class="arc-readout">
        <span class="readout-label">ALT</span>
        <span class="readout-value alt-value">{{ relativeAlt.toFixed(1) }}</span>
        <span class="readout-unit">m</span>
      </div>
    </div>

    <!-- 速度弧线表 -->
    <div class="arc-gauge">
      <svg viewBox="0 0 140 150" class="arc-svg">
        <path :d="spdBgPath" class="arc-bg" />
        <path v-if="spdFrac > 0" :d="spdArcPath" class="arc-fg spd-fg" />
        <g v-for="t in spdTicks" :key="'s' + t.v">
          <line :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2" class="arc-tick" :class="{ major: t.major }" />
          <text v-if="t.v === 0 || t.v === SPD_MAX" :x="t.lx" :y="t.ly" class="arc-label" text-anchor="middle">{{ t.v }}</text>
        </g>
      </svg>
      <div class="arc-readout">
        <span class="readout-label">SPD</span>
        <span class="readout-value spd-value">{{ groundspeed.toFixed(1) }}</span>
        <span class="readout-unit">m/s</span>
      </div>
    </div>

    <!-- 爬升率指示 -->
    <div class="climb-indicator">
      <span class="climb-arrow" :class="climbState">{{ climbArrow }}</span>
      <span class="climb-text">爬升率</span>
      <span class="climb-value">{{ climb.toFixed(1) }} m/s</span>
    </div>

    <!-- 油门条 -->
    <div class="throttle-row">
      <span class="throttle-label">油门</span>
      <div class="throttle-bar">
        <div class="throttle-fill" :style="{ width: throttlePct + '%' }"></div>
      </div>
      <span class="throttle-value">{{ throttle.toFixed(0) }}%</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    relativeAlt: number;
    groundspeed: number;
    climb: number;
    /** 油门百分比（0~100） */
    throttle?: number;
  }>(),
  { throttle: 0 },
);

/** 弧线表几何：圆心 (70, 82)，半径 62，240° 从 150° 扫到 390° */
const CX = 70;
const CY = 82;
const R = 62;
const START = 150;
const SWEEP = 240;
/** 高度量程（米） */
const ALT_MAX = 120;
/** 速度量程（m/s） */
const SPD_MAX = 60;

/** 高度占比（clamp 0~1） */
const altFrac = computed(() => Math.max(0, Math.min(1, props.relativeAlt / ALT_MAX)));
/** 速度占比（clamp 0~1） */
const spdFrac = computed(() => Math.max(0, Math.min(1, props.groundspeed / SPD_MAX)));
/** 油门百分比（clamp 0~100） */
const throttlePct = computed(() => Math.max(0, Math.min(100, props.throttle)));

/** 高度背景弧 */
const altBgPath = computed(() => arcPath(CX, CY, R, START, START + SWEEP));
/** 高度值弧（按占比收尾） */
const altArcPath = computed(() => arcPath(CX, CY, R, START, START + SWEEP * altFrac.value));
/** 速度背景弧 */
const spdBgPath = computed(() => arcPath(CX, CY, R, START, START + SWEEP));
/** 速度值弧 */
const spdArcPath = computed(() => arcPath(CX, CY, R, START, START + SWEEP * spdFrac.value));

/** 高度刻度（0~120，每 10 一格） */
const altTicks = computed(() => makeTicks(ALT_MAX));
/** 速度刻度（0~60，每 5 一格） */
const spdTicks = computed(() => makeTicks(SPD_MAX, 5));

/**
 * 爬升状态：上升绿 / 下降红 / 平飞灰
 */
const climbState = computed(() => {
  if (props.climb > 0.05) return "up";
  if (props.climb < -0.05) return "down";
  return "flat";
});

/** 爬升箭头符号 */
const climbArrow = computed(() => (climbState.value === "up" ? "▲" : climbState.value === "down" ? "▼" : "—"));

/**
 * 生成弧线刻度线数组（每 step 一格，30° 倍数为主刻度）
 *
 * @param max - 量程上限
 * @param step - 刻度间隔
 * @returns 刻度线坐标与端点标签坐标
 */
function makeTicks(max: number, step = 10) {
  const ticks: { v: number; major: boolean; x1: number; y1: number; x2: number; y2: number; lx: number; ly: number }[] = [];
  for (let v = 0; v <= max; v += step) {
    const a = ((-90 + START + (SWEEP * v) / max) * Math.PI) / 180;
    ticks.push({
      v,
      major: v % 30 === 0,
      x1: CX + R * Math.cos(a),
      y1: CY + R * Math.sin(a),
      x2: CX + (R - 6) * Math.cos(a),
      y2: CY + (R - 6) * Math.sin(a),
      lx: CX + (R - 18) * Math.cos(a),
      ly: CY + (R - 18) * Math.sin(a) + 3,
    });
  }
  return ticks;
}

/**
 * 圆弧路径（多段折线近似）
 *
 * @param cx - 圆心 x
 * @param cy - 圆心 y
 * @param r - 半径
 * @param start - 起始角度（度，0° 朝上，顺时针）
 * @param end - 结束角度（度）
 * @returns SVG path d 字符串
 */
function arcPath(cx: number, cy: number, r: number, start: number, end: number): string {
  const pts: string[] = [];
  const steps = Math.max(24, Math.round(end - start));
  for (let i = 0; i <= steps; i++) {
    const a = ((start + ((end - start) * i) / steps - 90) * Math.PI) / 180;
    pts.push(`${(cx + r * Math.cos(a)).toFixed(2)},${(cy + r * Math.sin(a)).toFixed(2)}`);
  }
  return `M ${pts.join(" L ")}`;
}
</script>

<style scoped>
.gauge-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  align-items: start;
}

.arc-gauge {
  position: relative;
  max-width: 215px;
  margin: 0 auto;
  width: 100%;
}

.arc-svg {
  width: 100%;
  display: block;
}

.arc-bg {
  fill: none;
  stroke: rgba(214, 230, 245, 0.07);
  stroke-width: 9;
  stroke-linecap: round;
}

.arc-fg {
  fill: none;
  stroke-width: 9;
  stroke-linecap: round;
}

.alt-fg {
  stroke: url(#qgc-alt-grad);
  filter: drop-shadow(0 0 4px rgba(0, 180, 216, 0.65));
}

.spd-fg {
  stroke: url(#qgc-spd-grad);
  filter: drop-shadow(0 0 4px rgba(0, 212, 170, 0.6));
}

.arc-tick {
  stroke: rgba(214, 230, 245, 0.35);
  stroke-width: 1;
}

.arc-tick.major {
  stroke: rgba(214, 230, 245, 0.75);
  stroke-width: 1.5;
}

.arc-label {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 9px;
  fill: #88a3c2;
}

.arc-readout {
  position: absolute;
  top: 55%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}

.readout-label {
  font-size: 9px;
  letter-spacing: 2.5px;
  color: #88a3c2;
}

.readout-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 24px;
  font-weight: 700;
  line-height: 1.1;
}

.alt-value {
  color: #7deeff;
  text-shadow: 0 0 8px rgba(0, 180, 216, 0.65);
}

.spd-value {
  color: #8affc4;
  text-shadow: 0 0 8px rgba(0, 212, 170, 0.6);
}

.readout-unit {
  font-size: 10px;
  color: #88a3c2;
}

.climb-indicator {
  grid-column: 1 / -1;
  justify-self: center;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: -2px;
}

.climb-arrow {
  font-size: 15px;
  line-height: 1;
}

.climb-arrow.up {
  color: var(--text-success, #00e676);
  text-shadow: 0 0 6px rgba(0, 230, 118, 0.8);
}

.climb-arrow.down {
  color: var(--text-danger, #ff3355);
  text-shadow: 0 0 6px rgba(255, 51, 85, 0.8);
}

.climb-arrow.flat {
  color: #88a3c2;
}

.climb-text {
  font-size: 12px;
  color: #88a3c2;
}

.climb-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 15px;
  color: #d6e6f5;
}

/* 油门条 */
.throttle-row {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  background: rgba(7, 13, 26, 0.5);
  border: 1px solid rgba(30, 58, 95, 0.6);
  border-radius: 6px;
}

.throttle-label {
  font-size: 12px;
  color: #88a3c2;
  letter-spacing: 1px;
}

.throttle-bar {
  flex: 1;
  height: 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  overflow: hidden;
}

.throttle-fill {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, #0077b6, #00d4ff);
  box-shadow: 0 0 8px rgba(0, 180, 216, 0.6);
  transition: width 0.2s ease;
}

.throttle-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 14px;
  font-weight: 700;
  color: #7deeff;
  min-width: 44px;
  text-align: right;
  text-shadow: 0 0 6px rgba(0, 180, 216, 0.5);
}
</style>
