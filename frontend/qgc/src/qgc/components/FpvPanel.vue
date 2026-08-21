<!--
  FPV 第一视角视频面板（qgc FpvPanel）

  模拟器无真实视频源，此处以合成地平仪（随横滚/俯仰滚动）模拟 FPV HUD，
  并明确标注「模拟器无视频源」。接入真实相机时替换为 video 流即可。
-->
<template>
  <div class="fpv-panel">
    <div class="fpv-stage">
      <canvas ref="cv" width="480" height="300" class="fpv-canvas"></canvas>
      <div class="fpv-nosignal">模拟器无视频源</div>
      <div class="fpv-cross"></div>
      <div class="fpv-readout">
        <span>ROLL {{ (roll * 57.3).toFixed(0) }}°</span>
        <span>PITCH {{ (pitch * 57.3).toFixed(0) }}°</span>
        <span>ALT {{ alt.toFixed(1) }}m</span>
        <span>SPD {{ spd.toFixed(1) }}m/s</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";

const qgcApi = createQgcApi();
const cv = ref<HTMLCanvasElement | null>(null);
const roll = ref(0);
const pitch = ref(0);
const alt = ref(0);
const spd = ref(0);
let timer: number | undefined;

function draw() {
  const c = cv.value;
  if (!c) return;
  const ctx = c.getContext("2d");
  if (!ctx) return;
  const w = c.width;
  const h = c.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#0a0e14";
  ctx.fillRect(0, 0, w, h);

  ctx.save();
  ctx.translate(w / 2, h / 2);
  ctx.rotate(roll.value);
  const pitchPx = pitch.value * 4; // 每度 4px
  // 天空
  ctx.fillStyle = "#1b3b6f";
  ctx.fillRect(-w, -h - pitchPx, w * 2, h);
  // 地面
  ctx.fillStyle = "#5a3a1b";
  ctx.fillRect(-w, pitchPx, w * 2, h);
  // 地平线
  ctx.strokeStyle = "#e0e0e0";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(-w, pitchPx);
  ctx.lineTo(w, pitchPx);
  ctx.stroke();
  // 俯仰刻度
  ctx.strokeStyle = "#9fe";
  ctx.lineWidth = 1;
  for (let p = -30; p <= 30; p += 10) {
    if (p === 0) continue;
    const y = pitchPx - p * 4;
    ctx.beginPath();
    ctx.moveTo(-20, y);
    ctx.lineTo(20, y);
    ctx.stroke();
  }
  ctx.restore();
}

async function poll() {
  try {
    const r = await qgcApi.getTelemetry();
    const t = (r.data ?? {}) as any;
    roll.value = t.roll ?? 0;
    pitch.value = t.pitch ?? 0;
    alt.value = t.alt_m ?? 0;
    spd.value = t.speed_m_s ?? 0;
  } catch {
    /* ignore */
  }
  draw();
}

onMounted(() => {
  timer = window.setInterval(poll, 150);
});
onUnmounted(() => {
  if (timer !== undefined) clearInterval(timer);
});
</script>

<style scoped>
.fpv-panel {
  display: flex;
  justify-content: center;
}
.fpv-stage {
  position: relative;
  width: 480px;
  max-width: 100%;
}
.fpv-canvas {
  width: 100%;
  border-radius: 6px;
  background: #0a0e14;
}
.fpv-nosignal {
  position: absolute;
  top: 8px;
  left: 0;
  right: 0;
  text-align: center;
  color: #ff7b7b;
  font-size: 12px;
  letter-spacing: 1px;
}
.fpv-cross::before,
.fpv-cross::after {
  content: "";
  position: absolute;
  background: rgba(255, 255, 255, 0.6);
}
.fpv-cross::before {
  left: 50%;
  top: 50%;
  width: 24px;
  height: 2px;
  transform: translate(-50%, -50%);
}
.fpv-cross::after {
  left: 50%;
  top: 50%;
  width: 2px;
  height: 24px;
  transform: translate(-50%, -50%);
}
.fpv-readout {
  position: absolute;
  bottom: 6px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-around;
  color: #7CFFB2;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
</style>
