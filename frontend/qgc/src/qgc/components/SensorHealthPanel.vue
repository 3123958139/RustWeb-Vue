<!--
  传感器健康面板（qgc SensorHealthPanel）

  解析 EKF_STATUS_REPORT 标志位与 VIBRATION 振动数据，展示估计器各项
  健康状态与三轴振动。对应后端 telemetry 的 ekf_* / vibration_* 字段
  （经 WS telemetry 事件实时推送）。
-->
<template>
  <div class="sensor-panel" v-if="telemetry">
    <div class="sp-section">
      <div class="sp-title">EKF 估计器状态</div>
      <div class="sp-grid">
        <div
          v-for="item in ekfItems"
          :key="item.name"
          class="sp-cell"
          :class="item.ok ? 'ok' : 'bad'"
        >
          <span class="sp-dot" />
          <span class="sp-label">{{ item.name }}</span>
        </div>
      </div>
    </div>

    <div class="sp-section">
      <div class="sp-title">核心方差（越小越健康）</div>
      <div class="sp-row">
        <span class="sp-name">速度</span><span class="sp-val">{{ fmt(telemetry.ekf_vel_variance) }}</span>
      </div>
      <div class="sp-row">
        <span class="sp-name">水平位置</span><span class="sp-val">{{ fmt(telemetry.ekf_pos_horiz_variance) }}</span>
      </div>
      <div class="sp-row">
        <span class="sp-name">垂直位置</span><span class="sp-val">{{ fmt(telemetry.ekf_pos_vert_variance) }}</span>
      </div>
      <div class="sp-row">
        <span class="sp-name">罗盘</span><span class="sp-val">{{ fmt(telemetry.ekf_compass_variance) }}</span>
      </div>
    </div>

    <div class="sp-section">
      <div class="sp-title">振动（m/s/s RMS）</div>
      <div class="sp-row">
        <span class="sp-name">X</span><span class="sp-val" :class="vibLevel(telemetry.vibration_x)">{{ fmt(telemetry.vibration_x) }}</span>
      </div>
      <div class="sp-row">
        <span class="sp-name">Y</span><span class="sp-val" :class="vibLevel(telemetry.vibration_y)">{{ fmt(telemetry.vibration_y) }}</span>
      </div>
      <div class="sp-row">
        <span class="sp-name">Z</span><span class="sp-val" :class="vibLevel(telemetry.vibration_z)">{{ fmt(telemetry.vibration_z) }}</span>
      </div>
    </div>
  </div>
  <div v-else class="sp-empty">等待遥测数据…</div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { QgcTelemetry } from "@/qgc/api/qgc";

const props = defineProps<{ telemetry?: Partial<QgcTelemetry> }>();

// EKF 估计器各项（MAV_ESTIMATOR_STATUS 位定义，位 0~11）
const EKF_BITS: { bit: number; name: string }[] = [
  { bit: 0, name: "姿态" },
  { bit: 1, name: "水平速度" },
  { bit: 2, name: "垂直速度" },
  { bit: 3, name: "相对水平位置" },
  { bit: 4, name: "绝对水平位置" },
  { bit: 5, name: "绝对垂直位置" },
  { bit: 6, name: "相对高度 AGL" },
  { bit: 7, name: "定点模式" },
  { bit: 8, name: "预测相对位置" },
  { bit: 9, name: "预测绝对位置" },
  { bit: 10, name: "GPS 故障保护" },
  { bit: 11, name: "加速度计误差" },
];

const ekfItems = computed(() => {
  const flags = props.telemetry?.ekf_flags ?? 0;
  return EKF_BITS.map((e) => ({ ...e, ok: ((flags >> e.bit) & 1) === 1 }));
});

function fmt(v?: number): string {
  return v == null ? "-" : v.toFixed(3);
}

// 振动分级：<0.2 正常，0.2~0.5 警告，>0.5 危险
function vibLevel(v?: number): string {
  if (v == null) return "";
  if (v > 0.5) return "bad";
  if (v > 0.2) return "warn";
  return "ok";
}
</script>

<style scoped>
.sensor-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.sp-section {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
  padding: 8px 10px;
}
.sp-title {
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--el-text-color-primary);
}
.sp-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px 10px;
}
.sp-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
.sp-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #67c23a;
}
.sp-cell.ok .sp-dot {
  background: #67c23a;
}
.sp-cell.bad .sp-dot {
  background: #f56c6c;
}
.sp-cell.bad .sp-label {
  color: #f56c6c;
}
.sp-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  padding: 2px 0;
}
.sp-val.ok {
  color: #67c23a;
}
.sp-val.warn {
  color: #e6a23c;
}
.sp-val.bad {
  color: #f56c6c;
}
.sp-empty {
  color: var(--el-text-color-secondary);
  padding: 20px;
  text-align: center;
}
</style>
