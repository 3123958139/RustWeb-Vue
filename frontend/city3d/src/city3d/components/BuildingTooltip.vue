<!--
  建筑信息浮窗（HUD 底部中央）

  悬停或点击建筑时展示详情：名称 / 区域 / 层数 / 高度 / 状态 / 能耗 / 人口 / 入住率
-->
<template>
  <transition name="fade-slide">
    <div v-if="building" class="building-tooltip glass-panel">
      <div class="tooltip-head">
        <span class="tooltip-name">{{ building.name }}</span>
        <el-tag :type="statusTagType(building.status)" size="small" effect="dark">
          {{ building.status }}
        </el-tag>
      </div>
      <div class="tooltip-meta">
        <span class="meta-chip">{{ districtName }}</span>
        <span class="meta-chip">{{ building.floors }} 层</span>
        <span class="meta-chip">{{ building.height }} m</span>
      </div>
      <div class="tooltip-rows">
        <div class="tooltip-row">
          <span class="row-label">实时能耗</span>
          <span class="row-value" style="color: #ffd166">{{ building.energy_kw }} kW</span>
        </div>
        <div class="tooltip-row">
          <span class="row-label">常驻人口</span>
          <span class="row-value" style="color: #8ae9c1">{{ building.population.toLocaleString() }} 人</span>
        </div>
        <div class="tooltip-row">
          <span class="row-label">入住率</span>
          <div class="occupancy-bar">
            <div
              class="occupancy-fill"
              :style="{ width: `${Math.round(building.occupancy * 100)}%`, background: occupancyColor(building.occupancy) }"
            ></div>
          </div>
          <span class="row-value">{{ Math.round(building.occupancy * 100) }}%</span>
        </div>
      </div>
      <div class="tooltip-coord">
        坐标 ({{ building.x }}, {{ building.z }})
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Building } from "@/city3d/api/city3d";

const props = defineProps<{
  building: Building | null;
  districtName?: string;
}>();

const districtName = computed(() => props.districtName || "未知区域");

function statusTagType(status: string): "success" | "warning" | "info" {
  if (status === "运行中") return "success";
  if (status === "维护中") return "warning";
  return "info";
}

function occupancyColor(value: number): string {
  if (value >= 0.85) return "#8ae9c1";
  if (value >= 0.65) return "#ffd166";
  return "#ff6b6b";
}
</script>

<style scoped>
.building-tooltip {
  position: absolute;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  width: 360px;
  padding: 14px 16px;
  z-index: 15;
  background: rgba(8, 14, 28, 0.75);
}

.glass-panel {
  border: 1px solid rgba(0, 212, 255, 0.25);
  border-radius: 12px;
  backdrop-filter: blur(14px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.tooltip-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
}

.tooltip-name {
  font-size: 16px;
  font-weight: 700;
  color: #e6f4ff;
  letter-spacing: 0.5px;
}

.tooltip-meta {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}

.meta-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  color: #9adcff;
  background: rgba(0, 212, 255, 0.1);
  border: 1px solid rgba(0, 212, 255, 0.2);
}

.tooltip-rows {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tooltip-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.row-label {
  font-size: 12px;
  color: rgba(160, 200, 240, 0.65);
  width: 60px;
  flex-shrink: 0;
}

.row-value {
  font-size: 12px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.occupancy-bar {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.08);
  overflow: hidden;
}

.occupancy-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}

.tooltip-coord {
  margin-top: 8px;
  font-size: 10px;
  color: rgba(160, 200, 240, 0.4);
  text-align: right;
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s ease;
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(12px);
}

@media (max-width: 700px) {
  .building-tooltip {
    width: calc(100% - 32px);
  }
}
</style>
