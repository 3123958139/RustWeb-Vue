<!--
  区域面板（HUD 右上角）

  列出城市区域及其建筑统计，点击区域让相机聚焦该区域。
-->
<template>
  <div class="district-panel glass-panel">
    <div class="panel-header">
      <span class="panel-title">城市区域</span>
      <span class="panel-sub">{{ districts.length }} 个分区</span>
    </div>
    <div class="district-list">
      <div
        v-for="d in districts"
        :key="d.district.id"
        class="district-item"
        :class="{ active: activeId === d.district.id }"
        @click="$emit('select', d.district.id)"
      >
        <span class="district-dot" :style="{ background: d.district.color, boxShadow: `0 0 8px ${d.district.color}` }"></span>
        <div class="district-info">
          <span class="district-name">{{ d.district.name }}</span>
          <span class="district-meta">{{ d.building_count }} 栋 · {{ Math.round(d.total_energy_kw) }} kW</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { DistrictSummary } from "@/city3d/api/city3d";

defineProps<{
  districts: DistrictSummary[];
  activeId?: string | null;
}>();

defineEmits<{
  (e: "select", id: string): void;
}>();
</script>

<style scoped>
.district-panel {
  position: absolute;
  top: 86px;
  right: 16px;
  width: 230px;
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
  margin-bottom: 10px;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: #e6f4ff;
  letter-spacing: 1px;
}

.panel-sub {
  font-size: 11px;
  color: rgba(160, 200, 240, 0.6);
}

.district-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 240px;
  overflow-y: auto;
}

.district-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.25s ease;
}

.district-item:hover {
  background: rgba(0, 212, 255, 0.08);
  border-color: rgba(0, 212, 255, 0.25);
}

.district-item.active {
  background: rgba(0, 212, 255, 0.14);
  border-color: rgba(0, 212, 255, 0.4);
}

.district-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.district-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.district-name {
  font-size: 13px;
  color: #dceeff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.district-meta {
  font-size: 11px;
  color: rgba(160, 200, 240, 0.55);
}

@media (max-width: 700px) {
  .district-panel {
    display: none;
  }
}
</style>
