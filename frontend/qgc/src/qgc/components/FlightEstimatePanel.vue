<!--
  航程 / 电量估算面板（qgc FlightEstimatePanel）

  基于当前任务航点（GET /api/qgc/mission）累加相邻点大圆距离，
  按默认巡航速度估算飞行时间与电量消耗。纯前端计算，无需后端新增接口。
-->
<template>
  <div class="est-panel" v-loading="loading">
    <el-button size="small" class="est-refresh" @click="load">重新计算</el-button>
    <div class="est-grid" v-if="stats">
      <div class="est-cell">
        <span class="est-name">航点数量</span>
        <span class="est-val">{{ stats.count }}</span>
      </div>
      <div class="est-cell">
        <span class="est-name">总航程</span>
        <span class="est-val">{{ distText }}</span>
      </div>
      <div class="est-cell">
        <span class="est-name">预计时间</span>
        <span class="est-val">{{ timeText }}</span>
      </div>
      <div class="est-cell">
        <span class="est-name">预计耗电</span>
        <span class="est-val">{{ batteryText }}</span>
      </div>
    </div>
    <div class="est-hint">按巡航速度 5 m/s、平均电流 2 A 估算（仅参考）</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";
import type { QgcMissionItem } from "@/qgc/api/qgc";

const qgcApi = createQgcApi();
const items = ref<QgcMissionItem[]>([]);
const loading = ref(false);

const CRUISE_SPEED = 5; // m/s
const AVG_CURRENT = 2; // A
const BATTERY_MAH = 5000; // mAh（以 5000mAh 典型 6S 估算）

onMounted(load);

async function load() {
  loading.value = true;
  try {
    const res = await qgcApi.getMission();
    if (res.data) items.value = res.data.items ?? [];
  } finally {
    loading.value = false;
  }
}

// 大圆距离（haversine），经纬度为度
function haversine(a: QgcMissionItem, b: QgcMissionItem): number {
  const R = 6371000; // 地球半径（米）
  const toRad = (d: number) => (d * Math.PI) / 180;
  const dLat = toRad((b.lat ?? 0) - (a.lat ?? 0));
  const dLon = toRad((b.lon ?? 0) - (a.lon ?? 0));
  const lat1 = toRad(a.lat ?? 0);
  const lat2 = toRad(b.lat ?? 0);
  const h =
    Math.sin(dLat / 2) ** 2 + Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) ** 2;
  return 2 * R * Math.asin(Math.min(1, Math.sqrt(h)));
}

const stats = computed(() => {
  const pts = items.value.filter((p) => p.lat != null && p.lon != null);
  let dist = 0;
  for (let i = 1; i < pts.length; i++) {
    dist += haversine(pts[i - 1], pts[i]);
  }
  const minutes = dist / (CRUISE_SPEED * 60);
  const mah = (dist / (CRUISE_SPEED * 3600)) * AVG_CURRENT * 1000;
  return { count: pts.length, dist, minutes, mah };
});

const distText = computed(() => {
  if (!stats.value) return "-";
  const d = stats.value.dist;
  return d > 1000 ? `${(d / 1000).toFixed(2)} km` : `${d.toFixed(0)} m`;
});

const timeText = computed(() => {
  if (!stats.value) return "-";
  return `${stats.value.minutes.toFixed(1)} 分钟`;
});

const batteryText = computed(() => {
  if (!stats.value) return "-";
  const pct = (stats.value.mah / BATTERY_MAH) * 100;
  return `${pct.toFixed(1)} %`;
});
</script>

<style scoped>
.est-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.est-refresh {
  align-self: flex-start;
}
.est-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.est-cell {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.est-name {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.est-val {
  font-size: 18px;
  font-weight: 600;
}
.est-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
