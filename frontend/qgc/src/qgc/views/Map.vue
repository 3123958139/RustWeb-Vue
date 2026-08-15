<!--
  地图与任务（qgc Map）

  Leaflet + OpenStreetMap 瓦片：
  1. 飞机 Marker（SVG 图标按航向旋转）+ 历史轨迹线（10Hz 遥测驱动）
  2. 点击地图添加航点（加入 MissionPanel 列表）
  3. 航点 Marker 同步显示，MissionPanel 管理增删/上传/下载/清除
  4. mission_progress 事件实时驱动面板进度
-->
<template>
  <div class="qgc-map-root">
    <div class="map-container" ref="mapEl"></div>
    <div class="map-sidebar">
      <MissionPanel ref="missionPanelRef" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import MissionPanel from "@/qgc/components/MissionPanel.vue";

/** 地图容器 DOM 引用 */
const mapEl = ref<HTMLElement | null>(null);
/** 任务面板引用（调用暴露方法） */
const missionPanelRef = ref<InstanceType<typeof MissionPanel> | null>(null);

/** Leaflet 地图实例 */
let map: L.Map | null = null;
/** 飞机 Marker */
let planeMarker: L.Marker | null = null;
/** 轨迹线 */
let trailLine: L.Polyline | null = null;
/** 航点 Marker 图层组 */
let waypointLayer: L.LayerGroup | null = null;
/** 历史轨迹点 */
const trailPoints: L.LatLngTuple[] = [];

/** 飞机 SVG 图标（按航向旋转，正北 0°） */
function planeIcon(heading: number): L.DivIcon {
  return L.divIcon({
    className: "qgc-plane-icon",
    html: `<svg width="36" height="36" viewBox="0 0 36 36" style="transform: rotate(${heading}deg)">
      <path d="M18 2 L22 18 L18 34 L14 18 Z" fill="#ffcc00" stroke="#000" stroke-width="1"/>
    </svg>`,
    iconSize: [36, 36],
    iconAnchor: [18, 18],
  });
}

/** 航点 Marker（首页金色圆点，普通航点蓝色菱形） */
function waypointIcon(seq: number): L.DivIcon {
  if (seq === 0) {
    return L.divIcon({
      className: "qgc-waypoint-icon",
      html: `<div class="wp-home"></div>`,
      iconSize: [14, 14],
      iconAnchor: [7, 7],
    });
  }
  return L.divIcon({
    className: "qgc-waypoint-icon",
    html: `<div class="wp-diamond"></div>`,
    iconSize: [12, 12],
    iconAnchor: [6, 6],
  });
}

/** 更新飞机位置与航向 */
function updatePlane(lat?: number, lon?: number, heading?: number) {
  if (map === null || lat === undefined || lon === undefined) return;
  const position: L.LatLngTuple = [lat, lon];
  if (planeMarker) {
    planeMarker.setLatLng(position);
    planeMarker.setIcon(planeIcon(heading ?? 0));
  } else {
    planeMarker = L.marker(position, { icon: planeIcon(heading ?? 0) }).addTo(map);
  }
  // 轨迹追加（与上一点距离 > 2 米才记录，避免静止堆积）
  const last = trailPoints[trailPoints.length - 1];
  if (!last || distanceMeters(last, position) > 2) {
    trailPoints.push(position);
    if (trailLine) {
      trailLine.setLatLngs(trailPoints);
    } else {
      trailLine = L.polyline(trailPoints, { color: "#ffcc00", weight: 2, opacity: 0.8 }).addTo(map);
    }
    if (trailPoints.length > 500) trailPoints.shift();
  }
}

/** 两点间距离（米，近似球面） */
function distanceMeters(a: L.LatLngTuple, b: L.LatLngTuple): number {
  const R = 6371000;
  const dLat = ((b[0] - a[0]) * Math.PI) / 180;
  const dLon = ((b[1] - a[1]) * Math.PI) / 180;
  const lat1 = (a[0] * Math.PI) / 180;
  const lat2 = (b[0] * Math.PI) / 180;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}

/** 刷新航点 Marker（与任务面板列表同步） */
function refreshWaypoints() {
  if (waypointLayer === null || missionPanelRef.value === null) return;
  waypointLayer.clearLayers();
  const items = missionPanelRef.value.items;
  items.forEach((item) => {
    L.marker([item.lat, item.lon], { icon: waypointIcon(item.seq) })
      .addTo(waypointLayer!)
      .bindTooltip(`#${item.seq} 高 ${item.altitude}m`, { permanent: false, direction: "top" });
  });
  // 首页（当前位置）在下载任务后由后端返回，刷新遥测时若任务列表含 seq=0 同步绘制
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  if (lat !== undefined && lon !== undefined) {
    L.marker([lat, lon], { icon: waypointIcon(0) })
      .addTo(waypointLayer!)
      .bindTooltip("首页", { permanent: false, direction: "top" });
  }
}

// ========== WebSocket 事件流 ==========

/** 事件流：遥测驱动飞机移动，任务进度驱动面板 */
const { telemetry, connect, disconnect } = useQgcEvents({
  onTelemetry: (t) => {
    updatePlane(t.lat, t.lon, t.heading);
  },
  onMissionProgress: (p) => {
    missionPanelRef.value?.applyProgress(p.state, p.total, p.received, p.result);
    // 下载/上传完成后刷新航点 Marker
    if (p.state === "idle") {
      refreshWaypoints();
    }
  },
});

onMounted(async () => {
  // 初始化地图（上海默认视野，尝试取最新遥测定位）
  map = L.map(mapEl.value!, {
    center: [31.2304, 121.4737],
    zoom: 15,
  });
  L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
    maxZoom: 19,
  }).addTo(map);
  waypointLayer = L.layerGroup().addTo(map);

  // 点击地图添加航点
  map.on("click", (e: L.LeafletMouseEvent) => {
    missionPanelRef.value?.addExternalItem(e.latlng.lat, e.latlng.lng);
    refreshWaypoints();
  });

  // 初始遥测快照（未启动服务时保持默认视野）
  try {
    const response = await qgcApi.getTelemetry();
    const t = response.data;
    if (t && t.lat !== 0 && t.lon !== 0) {
      map.setView([t.lat, t.lon], 15);
      updatePlane(t.lat, t.lon, t.heading);
    }
  } catch {
    // 忽略错误
  }

  connect();
  refreshWaypoints();
});

onUnmounted(() => {
  disconnect();
  map?.remove();
  map = null;
  planeMarker = null;
  trailLine = null;
  waypointLayer = null;
});
</script>

<style scoped>
.qgc-map-root {
  display: flex;
  gap: 12px;
  height: calc(100vh - 96px);
  padding: 16px;
  box-sizing: border-box;
  background: #17181b;
}

.map-container {
  flex: 1;
  border-radius: 6px;
  overflow: hidden;
  background: #1f2126;
  min-height: 0;
}

.map-sidebar {
  width: 460px;
  flex-shrink: 0;
  overflow: auto;
  background-color: #1f2126;
  border: 1px solid #2c2f36;
  border-radius: 6px;
  padding: 12px;
  box-sizing: border-box;
}
</style>

<style>
/* Leaflet 容器内图标（非 scoped，divIcon 是字符串 HTML） */
.qgc-plane-icon {
  background: transparent;
  border: none;
}

.qgc-waypoint-icon {
  background: transparent;
  border: none;
}

.wp-home {
  width: 14px;
  height: 14px;
  background: #ffcc00;
  border: 2px solid #000;
  border-radius: 50%;
  box-sizing: border-box;
}

.wp-diamond {
  width: 12px;
  height: 12px;
  background: #4aa3ff;
  border: 1px solid #fff;
  transform: rotate(45deg);
  box-sizing: border-box;
}

/* 深色地图底图下让 Leaflet 控件可见 */
.leaflet-container {
  background: #141518;
}
</style>
