<!--
  事件流面板（HUD 左下角）

  展示城市事件（5 秒轮询），按级别配色，最新事件置顶。
-->
<template>
  <div class="event-panel glass-panel">
    <div class="panel-header">
      <span class="panel-title">实时事件</span>
      <span class="panel-sub">每 5 秒刷新</span>
    </div>
    <div class="event-list">
      <div v-if="events.length === 0" class="event-empty">暂无事件</div>
      <div v-for="e in events.slice(0, 8)" :key="e.id" class="event-item">
        <span class="event-dot" :style="{ background: typeColor(e.type) }"></span>
        <div class="event-body">
          <div class="event-head">
            <span class="event-title">{{ e.title }}</span>
            <span class="event-time">{{ formatTime(e.created_at) }}</span>
          </div>
          <div v-if="e.description" class="event-desc">{{ e.description }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { CityEvent } from "@/city3d/api/city3d";

defineProps<{
  events: CityEvent[];
}>();

function typeColor(type: string): string {
  switch (type) {
    case "critical":
      return "#ff6b6b";
    case "warning":
      return "#ffb347";
    default:
      return "#00d4ff";
  }
}

function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
</script>

<style scoped>
.event-panel {
  position: absolute;
  left: 16px;
  bottom: 20px;
  width: 340px;
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

.event-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 220px;
  overflow-y: auto;
}

.event-empty {
  color: rgba(160, 200, 240, 0.5);
  font-size: 12px;
  padding: 8px 0;
}

.event-item {
  display: flex;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.event-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 5px;
  flex-shrink: 0;
}

.event-body {
  min-width: 0;
}

.event-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.event-title {
  font-size: 13px;
  color: #dceeff;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.event-time {
  font-size: 10px;
  color: rgba(160, 200, 240, 0.5);
  flex-shrink: 0;
}

.event-desc {
  font-size: 11px;
  color: rgba(160, 200, 240, 0.6);
  margin-top: 2px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

@media (max-width: 700px) {
  .event-panel {
    display: none;
  }
}
</style>
