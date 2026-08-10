<!--
  城市概览统计面板（HUD 左上角）

  展示后端 /api/city3d/overview 聚合数据，带数字滚动动画。
-->
<template>
  <div class="stat-panel glass-panel">
    <div class="panel-header">
      <span class="panel-title">城市概览</span>
      <span class="panel-badge">LIVE</span>
    </div>
    <div class="stat-grid">
      <div class="stat-item" v-for="item in stats" :key="item.label">
        <div class="stat-value" :style="{ color: item.color }">
          {{ formatNumber(item.value) }}{{ item.suffix }}
        </div>
        <div class="stat-label">{{ item.label }}</div>
      </div>
    </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { CityOverview } from "@/city3d/api/city3d";

const props = defineProps<{
  overview: CityOverview | null;
}>();

const stats = computed(() => {
  const o = props.overview;
  if (!o) return [];
  return [
    { label: "建筑总数", value: o.total_buildings, suffix: " 栋", color: "#66e0ff" },
    { label: "常驻人口", value: o.total_population, suffix: " 人", color: "#8ae9c1" },
    { label: "实时能耗", value: o.total_energy_kw, suffix: " kW", color: "#ffd166" },
    { label: "活跃建筑", value: o.active_buildings, suffix: " 栋", color: "#c792ff" },
    { label: "城市区域", value: o.total_districts, suffix: " 个", color: "#ff9d6b" },
    { label: "今日事件", value: o.total_events, suffix: " 条", color: "#ff6b6b" },
  ];
});

function formatNumber(value: number): string {
  return value.toLocaleString("zh-CN");
}
</script>

<style scoped>
.stat-panel {
  position: absolute;
  top: 86px;
  left: 16px;
  width: 320px;
  padding: 14px 16px;
  z-index: 10;
}

.glass-panel {
  background: rgba(8, 14, 28, 0.6);
  border: 1px solid rgba(0, 212, 255, 0.18);
  border-radius: 12px;
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: #e6f4ff;
  letter-spacing: 1px;
}

.panel-badge {
  font-size: 10px;
  letter-spacing: 1px;
  color: #04121f;
  background: linear-gradient(135deg, #00d4ff, #5a8cff);
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 700;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px 16px;
}

.stat-value {
  font-size: 20px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  text-shadow: 0 0 12px rgba(0, 212, 255, 0.35);
}

.stat-label {
  font-size: 11px;
  color: rgba(160, 200, 240, 0.65);
  margin-top: 2px;
}

.stat-footer {
  display: flex;
  gap: 6px;
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid rgba(0, 212, 255, 0.12);
}

.status-chip {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 8px;
  color: #cfe0f5;
  background: rgba(0, 212, 255, 0.1);
  border: 1px solid rgba(0, 212, 255, 0.2);
}

@media (max-width: 700px) {
  .stat-panel {
    width: 200px;
    top: 78px;
  }
  .stat-value {
    font-size: 15px;
  }
}
</style>
