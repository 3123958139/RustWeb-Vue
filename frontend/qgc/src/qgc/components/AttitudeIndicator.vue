<!--
  姿态仪（HUD 风格人工地平线）

  根据滚转角（roll）旋转地平线，根据俯仰角（pitch）平移地平线。
  表现力升级：
  1. 天/地渐变填充 + 发光地平线（裁剪在表盘圆内）
  2. 双侧俯仰刻度阶梯（±10/20/30° 带数值）
  3. 顶部滚转刻度弧（±60°）+ 滚转指示三角
  4. 发光机身十字 + 数字读数
  connected=false（飞控断连）时整表降透明度并去饱和。
-->
<template>
  <div class="attitude-indicator" :class="{ disconnected: !connected }">
    <svg viewBox="0 0 200 200" class="attitude-svg">
      <defs>
        <linearGradient id="qgc-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#0d2b56" />
          <stop offset="100%" stop-color="#2f7fd0" />
        </linearGradient>
        <linearGradient id="qgc-ground" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#96713f" />
          <stop offset="100%" stop-color="#33261a" />
        </linearGradient>
        <clipPath id="qgc-face">
          <circle cx="100" cy="100" r="89" />
        </clipPath>
      </defs>

      <!-- 表壳 -->
      <circle cx="100" cy="100" r="95" fill="#0a1428" stroke="rgba(0,180,216,0.4)" stroke-width="2" />
      <circle cx="100" cy="100" r="89" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="1" />

      <!-- 地平线组：随 roll 旋转、随 pitch 平移，裁剪在表盘圆内 -->
      <g :transform="`rotate(${roll} 100 100) translate(0 ${pitchOffset})`" clip-path="url(#qgc-face)">
        <!-- 天空 / 地面（大幅矩形保证旋转后仍有覆盖） -->
        <rect x="-80" y="-140" width="360" height="240" fill="url(#qgc-sky)" />
        <rect x="-80" y="100" width="360" height="240" fill="url(#qgc-ground)" />
        <!-- 地平线（亮线 + 光晕） -->
        <line x1="-80" y1="100" x2="280" y2="100" stroke="rgba(255,255,255,0.25)" stroke-width="7" />
        <line x1="-80" y1="100" x2="280" y2="100" stroke="#ffffff" stroke-width="2" />
        <!-- 俯仰刻度阶梯：左右两侧刻度线 + 数值 -->
        <g v-for="deg in pitchLadder" :key="deg">
          <line :x1="30" :y1="100 + deg * 2" :x2="80" :y2="100 + deg * 2" stroke="#e6f0f8" stroke-width="1.5" opacity="0.85" />
          <line :x1="120" :y1="100 + deg * 2" :x2="170" :y2="100 + deg * 2" stroke="#e6f0f8" stroke-width="1.5" opacity="0.85" />
          <text :x="25" :y="103 + deg * 2" class="pitch-label" text-anchor="end">{{ deg > 0 ? "+" + deg : deg }}</text>
          <text :x="175" :y="103 + deg * 2" class="pitch-label" text-anchor="start">{{ deg > 0 ? "+" + deg : deg }}</text>
        </g>
      </g>

      <!-- 滚转刻度弧（顶部固定，-60°~+60°） -->
      <path :d="bankArc" fill="none" stroke="#2c4a6e" stroke-width="1.5" />
      <g v-for="t in bankTicks" :key="t.roll">
        <line :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2" stroke="#5d7fa6" :stroke-width="t.major ? 2 : 1" />
        <text v-if="t.major && t.roll !== 0" :x="t.tx" :y="t.ty" class="bank-label" text-anchor="middle">{{ Math.abs(t.roll) }}</text>
      </g>

      <!-- 滚转指示三角（随 roll 绕中心旋转） -->
      <g :transform="`rotate(${roll} 100 100)`">
        <path d="M 100 14 L 95.5 25 L 104.5 25 Z" fill="#00b4d8" style="filter: drop-shadow(0 0 3px rgba(0,180,216,0.9));" />
      </g>

      <!-- 固定机身标记（发光十字） -->
      <line x1="62" y1="100" x2="88" y2="100" stroke="#ffd24a" stroke-width="2" />
      <line x1="112" y1="100" x2="138" y2="100" stroke="#ffd24a" stroke-width="2" />
      <path d="M 92 100 L 108 100 L 102 110 L 98 110 Z" fill="#ffd24a" style="filter: drop-shadow(0 0 4px rgba(255,210,74,0.8));" />
    </svg>

    <!-- 数字读数 -->
    <div class="attitude-values">
      <span class="value-block">
        <span class="value-label">ROLL</span>
        <span class="value">{{ roll.toFixed(1) }}°</span>
      </span>
      <span class="value-block">
        <span class="value-label">PITCH</span>
        <span class="value">{{ pitch.toFixed(1) }}°</span>
      </span>
    </div>

    <!-- 角速率条 -->
    <div class="attitude-rates">
      <span class="rate-item">
        <i class="rate-dot r"></i>
        <span class="rate-label">滚转</span>
        <span class="rate-value">{{ rollRate.toFixed(1) }}°/s</span>
      </span>
      <span class="rate-item">
        <i class="rate-dot p"></i>
        <span class="rate-label">俯仰</span>
        <span class="rate-value">{{ pitchRate.toFixed(1) }}°/s</span>
      </span>
      <span class="rate-item">
        <i class="rate-dot y"></i>
        <span class="rate-label">偏航</span>
        <span class="rate-value">{{ yawRate.toFixed(1) }}°/s</span>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    roll: number;
    pitch: number;
    /** 飞控是否连接（断连时整表置灰） */
    connected?: boolean;
    /** 滚转速率（度/秒） */
    rollRate?: number;
    /** 俯仰速率（度/秒） */
    pitchRate?: number;
    /** 偏航速率（度/秒） */
    yawRate?: number;
  }>(),
  { connected: true, rollRate: 0, pitchRate: 0, yawRate: 0 },
);

/** 俯仰角转换为像素平移（1 度 = 2px） */
const pitchOffset = computed(() => props.pitch * 2);

/** 俯仰刻度阶梯（度） */
const pitchLadder = [-30, -20, -10, 10, 20, 30];

/** 顶部滚转刻度弧路径（r=80，210°~330°） */
const bankArc = computed(() => polarPath(100, 100, 80, 210, 330));

/**
 * 滚转刻度：-60°~+60°，每 15° 一格，30° 的倍数为主刻度
 * 刻度线由 r=80 向外延伸到 r=88，数值放在 r=93 处
 */
const bankTicks = computed(() => {
  const ticks: { roll: number; major: boolean; x1: number; y1: number; x2: number; y2: number; tx: number; ty: number }[] = [];
  for (let r = -60; r <= 60; r += 15) {
    const a = ((-90 + r) * Math.PI) / 180;
    ticks.push({
      roll: r,
      major: r % 30 === 0,
      x1: 100 + 80 * Math.cos(a),
      y1: 100 + 80 * Math.sin(a),
      x2: 100 + 88 * Math.cos(a),
      y2: 100 + 88 * Math.sin(a),
      tx: 100 + 93 * Math.cos(a),
      ty: 100 + 93 * Math.sin(a) + 3,
    });
  }
  return ticks;
});

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
function polarPath(cx: number, cy: number, r: number, start: number, end: number): string {
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
.attitude-indicator {
  position: relative;
  width: 100%;
  max-width: 340px;
  margin: 0 auto;
}

.attitude-svg {
  width: 100%;
  display: block;
  filter: drop-shadow(0 4px 14px rgba(0, 0, 0, 0.45));
}

/* 飞控断连：整表置灰降透明度 */
.disconnected .attitude-svg {
  filter: grayscale(0.8) brightness(0.7);
}

.pitch-label,
.bank-label {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 8px;
  fill: #d6e6f5;
  opacity: 0.9;
}

.bank-label {
  fill: #88a3c2;
  font-size: 7px;
}

.attitude-values {
  position: absolute;
  bottom: 10px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-between;
  padding: 0 16px;
  pointer-events: none;
}

.value-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  background: rgba(7, 13, 26, 0.65);
  border: 1px solid rgba(0, 180, 216, 0.25);
  border-radius: 6px;
  padding: 3px 10px;
}

.value-label {
  font-size: 9px;
  letter-spacing: 2px;
  color: var(--text-dim, #7d94b5);
}

.value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 17px;
  color: #d6e6f5;
  text-shadow: 0 0 6px rgba(0, 180, 216, 0.55);
}

/* 角速率条 */
.attitude-rates {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-top: 8px;
}

.rate-item {
  display: flex;
  align-items: center;
  gap: 5px;
  flex: 1;
  justify-content: center;
  padding: 4px 6px;
  background: rgba(7, 13, 26, 0.6);
  border: 1px solid rgba(30, 58, 95, 0.6);
  border-radius: 6px;
}

.rate-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.rate-dot.r {
  background: var(--text-accent, #00b4d8);
  box-shadow: 0 0 5px rgba(0, 180, 216, 0.9);
}

.rate-dot.p {
  background: var(--text-success, #00e676);
  box-shadow: 0 0 5px rgba(0, 230, 118, 0.9);
}

.rate-dot.y {
  background: var(--text-hex, #f0c040);
  box-shadow: 0 0 5px rgba(240, 192, 64, 0.9);
}

.rate-label {
  font-size: 11px;
  color: var(--text-dim, #7d94b5);
}

.rate-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: #d6e6f5;
  white-space: nowrap;
}
</style>
