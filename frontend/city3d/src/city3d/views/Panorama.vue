<!--
  city3d 3D 全景展示页

  全屏 Three.js 城市场景 + 玻璃拟态 HUD 面板：
  - 顶部：时段 / 天气 / 热力 / 自动旋转控制
  - 左上：城市概览统计（后端聚合数据）
  - 右上：区域列表（点击聚焦）
  - 左下：实时事件流（5 秒轮询）
  - 底部：建筑信息浮窗（悬停/点击）
-->
<template>
  <div class="panorama-root">
    <!-- 3D 场景容器 -->
    <div ref="sceneHost" class="scene-host"></div>

    <!-- 加载遮罩 -->
    <transition name="fade">
      <div v-if="loading" class="loading-mask">
        <div class="loading-inner">
          <span class="loading-ring"></span>
          <span class="loading-text">城市数据加载中…</span>
        </div>
      </div>
    </transition>

    <!-- 错误提示 -->
    <div v-if="errorMessage" class="error-banner">
      <el-alert :title="errorMessage" type="error" :closable="false" show-icon />
    </div>

    <!-- HUD 顶部控制栏 -->
    <HudTopBar
      v-model:timeOfDay="timeOfDay"
      v-model:weather="weather"
      v-model:heatMode="heatMode"
      v-model:autoRotate="autoRotate"
      @reload="handleReload"
    />

    <!-- 城市概览 -->
    <StatPanel :overview="overview" />

    <!-- 区域列表 -->
    <DistrictPanel :districts="districts" :active-id="activeDistrictId" @select="handleSelectDistrict" />

    <!-- 事件流 -->
    <EventFeed :events="events" />

    <!-- 建筑详情浮窗 -->
    <BuildingTooltip :building="selectedBuilding" :district-name="selectedDistrictName" />

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
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Building } from "@/city3d/api/city3d";
import { useCityData } from "@/city3d/composables/useCityData";
import { createCityScene, type CitySceneHandle, type WeatherKey } from "@/city3d/composables/useCityScene";
import type { TimeOfDayKey } from "@/city3d/data/timeOfDay";
import HudTopBar from "@/city3d/components/HudTopBar.vue";
import StatPanel from "@/city3d/components/StatPanel.vue";
import DistrictPanel from "@/city3d/components/DistrictPanel.vue";
import EventFeed from "@/city3d/components/EventFeed.vue";
import BuildingTooltip from "@/city3d/components/BuildingTooltip.vue";

const sceneHost = ref<HTMLElement | null>(null);
const scene = ref<CitySceneHandle | null>(null);

const timeOfDay = ref<TimeOfDayKey>("night");
const weather = ref<WeatherKey>("none");
const heatMode = ref(false);
const autoRotate = ref(true);

const selectedBuilding = ref<Building | null>(null);
const activeDistrictId = ref<string | null>(null);

const {
  overview,
  districts,
  buildings,
  events,
  loading,
  errorMessage,
  loadAll,
  reloadSceneData,
  startEventPolling,
} = useCityData();

const selectedDistrictName = computed(() => {
  if (!selectedBuilding.value) return "";
  const district = districts.value.find((d) => d.district.id === selectedBuilding.value?.district_id);
  return district?.district.name ?? "";
});

function handleSelectBuilding(building: Building | null) {
  selectedBuilding.value = building;
  activeDistrictId.value = building ? building.district_id : null;
}

function handleSelectDistrict(districtId: string) {
  const district = districts.value.find((d) => d.district.id === districtId);
  if (!district) return;
  const districtBuildings = buildings.value.filter((b) => b.district_id === districtId);
  if (districtBuildings.length === 0) return;
  const cx = districtBuildings.reduce((sum, b) => sum + b.x, 0) / districtBuildings.length;
  const cz = districtBuildings.reduce((sum, b) => sum + b.z, 0) / districtBuildings.length;
  scene.value?.focusPoint(cx, cz);
  activeDistrictId.value = districtId;
}

async function handleReload() {
  loading.value = true;
  try {
    await reloadSceneData();
    scene.value?.updateBuildings(buildings.value, districts.value);
  } finally {
    loading.value = false;
  }
}

watch(timeOfDay, (key) => scene.value?.setTimeOfDay(key));
watch(weather, (key) => scene.value?.setWeather(key));
watch(heatMode, (enabled) => scene.value?.setHeatMode(enabled));
watch(autoRotate, (enabled) => scene.value?.setAutoRotate(enabled));

onMounted(async () => {
  await loadAll();
  startEventPolling();

  if (sceneHost.value) {
    scene.value = createCityScene({
      container: sceneHost.value,
      buildings: buildings.value,
      onSelectBuilding: handleSelectBuilding,
    });
  }
});

onBeforeUnmount(() => {
  scene.value?.dispose();
  scene.value = null;
});
</script>

<style scoped>
.panorama-root {
  position: relative;
  width: 100%;
  height: 100vh;
  overflow: hidden;
  background: #050b1f;
}

.scene-host {
  position: absolute;
  inset: 0;
}

.scene-host :deep(canvas) {
  display: block;
}

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

.error-banner {
  position: absolute;
  top: 86px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 30;
  width: 420px;
  max-width: calc(100% - 32px);
}

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

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@media (max-width: 700px) {
  .hint-bar {
    display: none;
  }
}
</style>
