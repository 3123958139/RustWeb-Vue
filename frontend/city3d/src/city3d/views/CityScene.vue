<!--
  CityScene - 城市 3D 数字孪生全景页

  全屏 Three.js 城市场景 + 玻璃拟态 HUD 面板：
  - 顶部：AppNavbar
  - 左上：城市名称发光标题
  - 右上：区域列表（带颜色指示器）
  - 左中：概览统计卡片（建筑数/人口/能耗/事件）
  - 右中：事件流（5 秒轮询）
  - 底部：控制栏（时段/天气/热力/自动旋转）
  - 悬停：建筑信息浮窗
  - 点击：建筑详情面板

  布局：全屏 3D 场景 + absolute 定位 HUD 覆盖层
-->
<template>
  <div class="city3d-root">
    <!-- 全局导航条由 App.vue 渲染（登录页除外） -->

    <!-- 3D 场景容器 -->
    <div ref="sceneContainer" class="scene-container"></div>

    <!-- 加载遮罩 -->
    <transition name="fade">
      <div v-if="dataLoading" class="loading-mask">
        <div class="loading-inner">
          <span class="loading-ring"></span>
          <span class="loading-text">城市数据加载中…</span>
        </div>
      </div>
    </transition>

    <!-- 错误提示 -->
    <div v-if="errorMessage || sceneError" class="error-banner">
      <el-alert :title="errorMessage || sceneError" type="error" :closable="false" show-icon />
    </div>

    <!-- 左上：城市名称 -->
    <div class="city-title">
      <span class="city-title-glow">智慧城市数字孪生</span>
      <span class="city-title-sub">SMART CITY DIGITAL TWIN</span>
    </div>

    <!-- 右上：区域列表 -->
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
          :class="{ active: activeDistrictId === d.district.id }"
          @click="focusDistrict(d.district.id)"
        >
          <span
            class="district-dot"
            :style="{ background: d.district.color, boxShadow: `0 0 8px ${d.district.color}` }"
          ></span>
          <div class="district-info">
            <span class="district-name">{{ d.district.name }}</span>
            <span class="district-meta">{{ d.building_count }} 栋</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 左中：概览统计 -->
    <div class="stats-panel glass-panel">
      <div class="panel-header">
        <span class="panel-title">城市概览</span>
        <span class="panel-badge">LIVE</span>
      </div>
      <div class="stats-grid">
        <div v-for="item in statItems" :key="item.label" class="stat-card">
          <div class="stat-value" :style="{ color: item.color }">
            {{ item.prefix }}{{ formatNumber(item.value) }}{{ item.suffix }}
          </div>
          <div class="stat-label">{{ item.label }}</div>
        </div>
      </div>
    </div>

    <!-- 右中：事件流 -->
    <div class="event-panel glass-panel">
      <div class="panel-header">
        <span class="panel-title">实时事件</span>
        <span class="panel-sub">每 5 秒刷新</span>
      </div>
      <div class="event-list">
        <div v-if="events.length === 0" class="event-empty">暂无事件</div>
        <div v-for="e in events.slice(0, 5)" :key="e.id" class="event-item">
          <span class="event-dot" :style="{ background: eventTypeColor(e.type) }"></span>
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

    <!-- 底部控制栏 -->
    <div class="control-bar glass-panel">
      <!-- 时段 -->
      <div class="control-group">
        <span class="control-label">时段</span>
        <div class="segmented">
          <button
            v-for="item in TIME_OF_DAY_KEYS"
            :key="item"
            class="segment"
            :class="{ active: timeOfDay === item }"
            @click="switchTimeOfDay(item)"
          >
            {{ TIME_OF_DAY[item].label }}
          </button>
        </div>
      </div>

      <!-- 天气 -->
      <div class="control-group">
        <span class="control-label">天气</span>
        <div class="segmented">
          <button
            v-for="item in weatherOptions"
            :key="item.key"
            class="segment"
            :class="{ active: currentWeather === item.key }"
            @click="switchWeather(item.key)"
          >
            {{ item.label }}
          </button>
        </div>
      </div>

      <!-- 模式开关 -->
      <div class="control-group switch-group">
        <el-switch :model-value="heatMode" size="small" @update:model-value="toggleHeatmap" />
        <span class="control-label">能耗热力</span>
      </div>
      <div class="control-group switch-group">
        <el-switch :model-value="autoRotateActive" size="small" @update:model-value="toggleAutoRotate" />
        <span class="control-label">自动旋转</span>
      </div>
    </div>

    <!-- 悬停浮窗 -->
    <transition name="fade-slide">
      <div v-if="sceneHoveredBuilding && !sceneSelectedBuilding" class="hover-tooltip glass-panel">
        <div class="tooltip-head">
          <span class="tooltip-name">{{ sceneHoveredBuilding.name }}</span>
          <el-tag :type="statusTagType(sceneHoveredBuilding.status)" size="small" effect="dark">
            {{ sceneHoveredBuilding.status }}
          </el-tag>
        </div>
        <div class="tooltip-meta">
          <span class="meta-chip">{{ getDistrictName(sceneHoveredBuilding.district_id) }}</span>
          <span class="meta-chip">{{ sceneHoveredBuilding.floors }} 层</span>
          <span class="meta-chip">{{ sceneHoveredBuilding.height }}m</span>
        </div>
        <div class="tooltip-rows">
          <div class="tooltip-row">
            <span class="row-label">能耗</span>
            <span class="row-value" style="color: #ffd166">{{ sceneHoveredBuilding.energy_kw }} kW</span>
          </div>
          <div class="tooltip-row">
            <span class="row-label">人口</span>
            <span class="row-value" style="color: #8ae9c1">{{ sceneHoveredBuilding.population.toLocaleString() }} 人</span>
          </div>
        </div>
      </div>
    </transition>

    <!-- 点击详情面板 -->
    <transition name="fade-slide">
      <div v-if="sceneSelectedBuilding" class="detail-panel glass-panel">
        <div class="tooltip-head">
          <span class="tooltip-name">{{ sceneSelectedBuilding.name }}</span>
          <el-tag :type="statusTagType(sceneSelectedBuilding.status)" size="small" effect="dark">
            {{ sceneSelectedBuilding.status }}
          </el-tag>
        </div>
        <div class="tooltip-meta">
          <span class="meta-chip">{{ getDistrictName(sceneSelectedBuilding.district_id) }}</span>
          <span class="meta-chip">{{ sceneSelectedBuilding.floors }} 层</span>
          <span class="meta-chip">{{ sceneSelectedBuilding.height }} m</span>
        </div>
        <div class="tooltip-rows">
          <div class="tooltip-row">
            <span class="row-label">实时能耗</span>
            <span class="row-value" style="color: #ffd166">{{ sceneSelectedBuilding.energy_kw }} kW</span>
          </div>
          <div class="tooltip-row">
            <span class="row-label">常驻人口</span>
            <span class="row-value" style="color: #8ae9c1">{{ sceneSelectedBuilding.population.toLocaleString() }} 人</span>
          </div>
          <div class="tooltip-row">
            <span class="row-label">入住率</span>
            <div class="occupancy-bar">
              <div
                class="occupancy-fill"
                :style="{
                  width: `${Math.round(sceneSelectedBuilding.occupancy * 100)}%`,
                  background: occupancyColor(sceneSelectedBuilding.occupancy),
                }"
              ></div>
            </div>
            <span class="row-value">{{ Math.round(sceneSelectedBuilding.occupancy * 100) }}%</span>
          </div>
        </div>
        <div class="tooltip-coord">坐标 ({{ sceneSelectedBuilding.x }}, {{ sceneSelectedBuilding.z }})</div>
      </div>
    </transition>

    <!-- 操作提示 -->
    <div class="hint-bar">
      <span>拖拽旋转</span><i></i>
      <span>滚轮缩放</span><i></i>
      <span>悬停查看建筑</span><i></i>
      <span>点击聚焦</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useCityData } from "@/city3d/composables/useCityData";
import { useCityScene, type WeatherKey } from "@/city3d/composables/useCityScene";
import { TIME_OF_DAY, TIME_OF_DAY_KEYS, type TimeOfDayKey } from "@/city3d/data/timeOfDay";

// ============ 场景 ============
const sceneContainer = ref<HTMLElement | null>(null);
const scene = useCityScene();
const sceneHoveredBuilding = computed(() => scene.hoveredBuilding.value);
const sceneSelectedBuilding = computed(() => scene.selectedBuilding.value);
const sceneError = computed(() => scene.sceneError.value);

// ============ 数据 ============
// 复用 useCityData 组合式函数（单一数据源 + 单一 5s 事件轮询定时器），
// 替代本文件此前内联的重复实现，避免双重轮询
const {
  overview,
  districts,
  buildings,
  events,
  loading: dataLoading,
  errorMessage,
  loadAll,
  startEventPolling,
  stopEventPolling,
} = useCityData(5000, { silentEventErrors: true });

// 首帧渲染前显示加载遮罩（对齐原实现 dataLoading 初始为 true）
dataLoading.value = true;

// ============ 控制 ============
const timeOfDay = ref<TimeOfDayKey>("night");
const currentWeather = ref<WeatherKey>("none");
const heatMode = ref(false);
const autoRotateActive = ref(true);
const activeDistrictId = ref<string | null>(null);

const weatherOptions: { key: WeatherKey; label: string }[] = [
  { key: "none", label: "晴朗" },
  { key: "rain", label: "降雨" },
  { key: "snow", label: "降雪" },
  { key: "fog", label: "浓雾" },
];

function switchTimeOfDay(key: TimeOfDayKey) {
  timeOfDay.value = key;
  scene.setTimeOfDay(key);
}

function switchWeather(key: WeatherKey) {
  currentWeather.value = key;
  scene.setWeather(key);
}

function toggleHeatmap(val: boolean) {
  heatMode.value = val;
  scene.setHeatmapMode(val);
}

function toggleAutoRotate(val: boolean) {
  autoRotateActive.value = val;
  if (val) scene.startAutoRotate();
  else scene.stopAutoRotate();
}

function focusDistrict(districtId: string) {
  const districtBuildings = buildings.value.filter((b) => b.district_id === districtId);
  if (districtBuildings.length === 0) return;
  // 使用第一个建筑作为聚焦目标
  scene.focusOnBuilding(districtBuildings[0].id);
  activeDistrictId.value = districtId;
}

// ============ 工具 ============
const statItems = computed(() => {
  const o = overview.value;
  if (!o) return [];
  return [
    { label: "建筑总数", value: o.total_buildings, suffix: " 栋", color: "#66e0ff", prefix: "" },
    { label: "常驻人口", value: o.total_population, suffix: " 人", color: "#8ae9c1", prefix: "" },
    { label: "实时能耗", value: o.total_energy_kw, suffix: " kW", color: "#ffd166", prefix: "" },
    { label: "今日事件", value: o.total_events, suffix: " 条", color: "#ff6b6b", prefix: "" },
  ];
});

function formatNumber(value: number): string {
  return value.toLocaleString("zh-CN");
}

function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function eventTypeColor(type: string): string {
  switch (type) {
    case "critical":
      return "#ff6b6b";
    case "warning":
      return "#ffb347";
    default:
      return "#00d4ff";
  }
}

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

function getDistrictName(districtId: string): string {
  const d = districts.value.find((d) => d.district.id === districtId);
  return d?.district.name ?? "未知区域";
}

// ============ 生命周期 ============
onMounted(async () => {
  await loadAll();
  startEventPolling();

  if (sceneContainer.value) {
    scene.initScene(sceneContainer.value);
    scene.loadBuildings(buildings.value);
  }
});

onBeforeUnmount(() => {
  stopEventPolling();
  scene.disposeScene();
});
</script>

<style scoped>
.city3d-root {
  position: relative;
  width: 100%;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: #050b1f;
  font-family: "Helvetica Neue", Helvetica, "PingFang SC", "Microsoft YaHei", sans-serif;
}

/* ============ 场景容器 ============ */
.scene-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1;
}

.scene-container :deep(canvas) {
  display: block;
}

/* ============ 加载遮罩 ============ */
.loading-mask {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(5, 11, 31, 0.72);
  backdrop-filter: blur(6px);
  z-index: 40;
}

.loading-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.loading-ring {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 3px solid rgba(0, 212, 255, 0.15);
  border-top-color: #00d4ff;
  animation: spin 0.9s linear infinite;
}

.loading-text {
  color: #9adcff;
  font-size: 13px;
  letter-spacing: 2px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* ============ 错误提示 ============ */
.error-banner {
  position: absolute;
  top: 86px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 30;
  width: 420px;
  max-width: calc(100% - 32px);
}

/* ============ 玻璃拟态面板 ============ */
.glass-panel {
  background: rgba(10, 14, 26, 0.7);
  border: 1px solid rgba(0, 212, 255, 0.2);
  border-radius: 12px;
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45);
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

.panel-badge {
  font-size: 10px;
  letter-spacing: 1px;
  color: #04121f;
  background: linear-gradient(135deg, #00d4ff, #5a8cff);
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 700;
}

/* ============ 左上：城市标题 ============ */
.city-title {
  position: absolute;
  top: 72px;
  left: 20px;
  z-index: 10;
  display: flex;
  flex-direction: column;
  gap: 2px;
  pointer-events: none;
}

.city-title-glow {
  font-size: 26px;
  font-weight: 800;
  letter-spacing: 4px;
  background: linear-gradient(90deg, #66e0ff, #9a8cff, #66e0ff);
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  animation: glowShift 4s ease-in-out infinite;
  text-shadow: 0 0 30px rgba(0, 212, 255, 0.3);
}

@keyframes glowShift {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}

.city-title-sub {
  font-size: 10px;
  color: rgba(160, 200, 240, 0.5);
  letter-spacing: 3px;
}

/* ============ 右上：区域列表 ============ */
.district-panel {
  position: absolute;
  top: 72px;
  right: 16px;
  width: 210px;
  padding: 14px 14px;
  z-index: 10;
}

.district-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  max-height: 260px;
  overflow-y: auto;
}

.district-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
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

/* ============ 左中：概览统计 ============ */
.stats-panel {
  position: absolute;
  top: 160px;
  left: 16px;
  width: 240px;
  padding: 14px 16px;
  z-index: 10;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px 14px;
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

/* ============ 右中：事件流 ============ */
.event-panel {
  position: absolute;
  right: 16px;
  top: 400px;
  width: 280px;
  padding: 14px 14px;
  z-index: 10;
}

.event-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 200px;
  overflow-y: auto;
}

.event-empty {
  color: rgba(160, 200, 240, 0.5);
  font-size: 12px;
  padding: 8px 0;
}

.event-item {
  display: flex;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.event-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 4px;
  flex-shrink: 0;
}

.event-body {
  min-width: 0;
  flex: 1;
}

.event-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.event-title {
  font-size: 12px;
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

/* ============ 底部控制栏 ============ */
.control-bar {
  position: absolute;
  left: 50%;
  bottom: 56px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 18px;
  z-index: 10;
  flex-wrap: wrap;
  justify-content: center;
}

.control-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.control-label {
  font-size: 12px;
  color: rgba(160, 200, 240, 0.75);
  white-space: nowrap;
}

.switch-group {
  gap: 5px;
}

.segmented {
  display: flex;
  gap: 2px;
  padding: 3px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(0, 212, 255, 0.15);
  border-radius: 8px;
}

.segment {
  padding: 4px 10px;
  font-size: 12px;
  color: rgba(200, 225, 250, 0.7);
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.25s ease;
  white-space: nowrap;
}

.segment:hover {
  color: #fff;
  background: rgba(0, 212, 255, 0.12);
}

.segment.active {
  color: #04121f;
  background: linear-gradient(135deg, #00d4ff, #5a8cff);
  font-weight: 600;
  box-shadow: 0 0 12px rgba(0, 212, 255, 0.45);
}

/* ============ 悬停/详情面板 ============ */
.hover-tooltip {
  position: absolute;
  left: 50%;
  bottom: 110px;
  transform: translateX(-50%);
  width: 320px;
  padding: 12px 14px;
  z-index: 15;
}

.detail-panel {
  position: absolute;
  left: 50%;
  bottom: 110px;
  transform: translateX(-50%);
  width: 360px;
  padding: 14px 16px;
  z-index: 15;
}

.tooltip-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 6px;
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
  margin-bottom: 8px;
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
  gap: 5px;
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
  margin-top: 6px;
  font-size: 10px;
  color: rgba(160, 200, 240, 0.4);
  text-align: right;
}

/* ============ 操作提示 ============ */
.hint-bar {
  position: absolute;
  left: 50%;
  bottom: 12px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11px;
  color: rgba(160, 200, 240, 0.45);
  z-index: 5;
  pointer-events: none;
  white-space: nowrap;
}

.hint-bar i {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: rgba(160, 200, 240, 0.4);
}

/* ============ 过渡动画 ============ */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
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

/* ============ 响应式 ============ */
@media (max-width: 768px) {
  .district-panel,
  .event-panel {
    display: none;
  }

  .stats-panel {
    width: 180px;
    top: 130px;
  }

  .stats-grid {
    grid-template-columns: 1fr;
    gap: 6px;
  }

  .stat-value {
    font-size: 16px;
  }

  .city-title-glow {
    font-size: 18px;
  }

  .control-bar {
    bottom: 44px;
    gap: 8px;
    padding: 8px 12px;
    width: calc(100% - 32px);
  }

  .hover-tooltip,
  .detail-panel {
    width: calc(100% - 32px);
    bottom: 90px;
  }

  .hint-bar {
    display: none;
  }
}
</style>