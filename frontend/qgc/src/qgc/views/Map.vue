<!--
  地图与任务（qgc Map）

  Leaflet + OpenStreetMap 瓦片：
  1. 飞机 Marker（SVG 图标按航向旋转）+ 历史轨迹线（10Hz 遥测驱动）
  2. 点击地图添加航点（加入 MissionPanel 列表）；随点随行模式下点击即飞向目标
  3. 航点 Marker 同步显示，MissionPanel 管理增删/上传/下载/清除
  4. mission_progress 事件实时驱动面板进度
  5. 视角控制按钮组（跟随飞机 / 回中复位 / 缩放）+ 飞机 HUD 叠加条

  说明：地图页为交互式页面，不做 1920×1080 整屏缩放（transform 会破坏
  Leaflet 鼠标坐标换算），改用与仪表盘一致的主题配色与面板风格。
-->
<template>
  <div class="qgc-map-root">
    <div class="map-stage">
      <!-- 地图上方状态条 -->
      <div class="map-topbar">
        <span class="topbar-brand">
          <span class="brand-dot"></span>
          <span class="topbar-title">地图与任务</span>
        </span>
        <span class="topbar-hint"><i class="hint-dot"></i>{{ planMode === "plan" ? "点击地图添加航点" : "随点随行：点击地图即飞向目标" }}</span>
        <div class="mode-switch">
          <button class="mode-btn" :class="{ active: planMode === 'plan' }" @click="planMode = 'plan'">航线规划</button>
          <button class="mode-btn" :class="{ active: planMode === 'goto' }" @click="planMode = 'goto'">随点随行</button>
        </div>
        <el-button size="small" class="offline-map-btn" @click="openOfflinePanel">离线地图</el-button>
      </div>
      <div class="map-body">
        <div class="map-container" ref="mapEl"></div>

        <!-- 视角控制按钮组 -->
        <div class="view-ctrl">
          <button class="vc-btn" :class="{ active: followPlane }" title="跟随飞机" @click="toggleFollow">◎</button>
          <button class="vc-btn" title="回中复位" @click="resetView">⌂</button>
          <button class="vc-btn" title="放大" @click="zoomBy(1)">+</button>
          <button class="vc-btn" title="缩小" @click="zoomBy(-1)">−</button>
        </div>

        <!-- 飞机 HUD 叠加条 -->
        <div class="hud-strip">
          <span class="hud-item"><i class="hud-dot alt"></i>高 <b>{{ (telemetry.relative_alt ?? 0).toFixed(1) }}m</b></span>
          <span class="hud-item"><i class="hud-dot spd"></i>速 <b>{{ (telemetry.groundspeed ?? 0).toFixed(1) }}m/s</b></span>
          <span class="hud-item"><i class="hud-dot hdg"></i>向 <b>{{ (telemetry.heading ?? 0).toFixed(0) }}°</b></span>
          <span class="hud-item"><i class="hud-dot dist"></i>距 <b>{{ ((telemetry.distance_home ?? 0) / 1000).toFixed(2) }}km</b></span>
        </div>
      </div>
    </div>
    <div class="map-sidebar">
      <MissionPanel ref="missionPanelRef" />
    </div>

    <!-- 离线地图面板（瓦片离线保存 / 加载管理） -->
    <el-dialog v-model="offlinePanelVisible" title="离线地图" width="560px" append-to-body class="offline-dialog">
      <OfflineMapPanel :center="mapCenter" />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import { getSessionToken } from "@shared";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import MissionPanel from "@/qgc/components/MissionPanel.vue";
import OfflineMapPanel from "@/qgc/components/OfflineMapPanel.vue";

/** 地图容器 DOM 引用 */
const mapEl = ref<HTMLElement | null>(null);
/** 任务面板引用（调用暴露方法） */
const missionPanelRef = ref<InstanceType<typeof MissionPanel> | null>(null);
/** 离线地图面板开关 */
const offlinePanelVisible = ref(false);
/** 离线面板初始中心（打开时取地图当前中心） */
const mapCenter = ref<[number, number]>([31.2304, 121.4737]);

/** 打开离线地图面板（中心点同步为地图当前中心） */
function openOfflinePanel() {
  const c = map?.getCenter();
  if (c) mapCenter.value = [c.lat, c.lng];
  offlinePanelVisible.value = true;
}

// ---------- 规划 / 随行模式与视角控制 ----------

/** 地图交互模式：plan 添加航点 / goto 随点随行 */
const planMode = ref<"plan" | "goto">("plan");
/** 跟随飞机开关 */
const followPlane = ref(false);
/** 随行目标高度（米） */
const gotoAlt = ref(30);

/** 跟随飞机 */
function toggleFollow() {
  followPlane.value = !followPlane.value;
  if (followPlane.value && telemetry.value.lat !== undefined && telemetry.value.lon !== undefined) {
    map?.panTo([telemetry.value.lat, telemetry.value.lon]);
  }
}

/** 回中复位 */
function resetView() {
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  if (lat !== undefined && lat !== 0 && lon !== undefined) {
    map?.setView([lat, lon], 15);
  } else {
    map?.setView([31.2304, 121.4737], 15);
  }
}

/** 缩放 */
function zoomBy(delta: number) {
  const z = (map?.getZoom() ?? 15) + delta;
  map?.setZoom(z);
}

/** 随点随行：点击地图飞向目标 */
async function onMapClick(lat: number, lon: number) {
  try {
    const response = await qgcApi.sendCommand("click_to_go", null, [lat, lon, gotoAlt.value]);
    if (!response.data) {
      ElMessage.error("随点随行指令发送失败");
    } else {
      ElMessage.success(`已下达随行指令 (${lat.toFixed(5)}, ${lon.toFixed(5)}, ${gotoAlt.value}m)`);
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "随点随行失败");
  }
}

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
    if (followPlane.value && t.lat !== undefined && t.lon !== undefined) {
      map?.panTo([t.lat, t.lon]);
    }
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
  // 瓦片经后端代理加载（磁盘缓存，离线可用）；token 经查询参数传递（img 无法带 Bearer 头）
  L.tileLayer(`/api/qgc/tiles/{z}/{x}/{y}?token=${encodeURIComponent(getSessionToken() ?? "")}`, {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
    maxZoom: 19,
  }).addTo(map);
  waypointLayer = L.layerGroup().addTo(map);

  // 点击地图：规划模式添加航点 / 随行模式下达飞行指令
  map.on("click", (e: L.LeafletMouseEvent) => {
    if (planMode.value === "goto") {
      onMapClick(e.latlng.lat, e.latlng.lng);
    } else {
      missionPanelRef.value?.addExternalItem(e.latlng.lat, e.latlng.lng);
      refreshWaypoints();
    }
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
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 14px;
  padding: 14px 16px;
  box-sizing: border-box;
  background:
    radial-gradient(1100px 380px at 50% -10%, rgba(0, 180, 216, 0.09), transparent 60%),
    var(--bg-page);
  overflow: hidden;
}

.map-stage {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.map-topbar {
  flex-shrink: 0;
  height: 52px;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 0 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.topbar-brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-accent);
  box-shadow: 0 0 10px rgba(0, 180, 216, 0.9);
  animation: brand-breathe 2.4s ease-in-out infinite;
}

.topbar-title {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 2px;
  background: linear-gradient(90deg, #00b4d8, #4d9fff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.topbar-hint {
  margin-left: auto;
  font-size: 12px;
  letter-spacing: 1px;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  gap: 8px;
}

.hint-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-success);
  box-shadow: 0 0 6px rgba(0, 230, 118, 0.8);
}

.map-container {
  position: absolute;
  inset: 0;
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-cell);
  border: 1px solid var(--border-color);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.map-body {
  flex: 1;
  min-height: 0;
  position: relative;
}

.map-sidebar {
  width: 620px;
  flex-shrink: 0;
  overflow: auto;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 14px;
  box-sizing: border-box;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

/* 视角控制按钮组 */
.view-ctrl {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  background: rgba(10, 20, 40, 0.75);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}

.vc-btn {
  width: 30px;
  height: 30px;
  border-radius: 6px;
  border: 1px solid rgba(0, 180, 216, 0.3);
  background: var(--btn-bg);
  color: var(--btn-text);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s ease;
}

.vc-btn:hover {
  background: var(--btn-hover-bg);
  border-color: rgba(0, 180, 216, 0.6);
  box-shadow: 0 0 10px rgba(0, 180, 216, 0.3);
}

.vc-btn.active {
  background: linear-gradient(180deg, #00b4d8, #0077b6);
  border-color: rgba(0, 180, 216, 0.8);
  color: #ffffff;
  box-shadow: 0 0 12px rgba(0, 180, 216, 0.5);
}

/* 飞机 HUD 叠加条 */
.hud-strip {
  position: absolute;
  bottom: 12px;
  left: 12px;
  z-index: 1000;
  display: flex;
  gap: 18px;
  padding: 8px 14px;
  border-radius: 8px;
  background: rgba(10, 20, 40, 0.75);
  border: 1px solid var(--border-color);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}

.hud-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-dim);
}

.hud-item b {
  font-family: "Consolas", "Courier New", monospace;
  color: var(--text-primary);
  text-shadow: 0 0 6px rgba(0, 180, 216, 0.4);
}

.hud-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.hud-dot.alt {
  background: #00d4ff;
  box-shadow: 0 0 5px rgba(0, 212, 255, 0.9);
}

.hud-dot.spd {
  background: #00e676;
  box-shadow: 0 0 5px rgba(0, 230, 118, 0.9);
}

.hud-dot.hdg {
  background: #f0c040;
  box-shadow: 0 0 5px rgba(240, 192, 64, 0.9);
}

.hud-dot.dist {
  background: #00d4aa;
  box-shadow: 0 0 5px rgba(0, 212, 170, 0.9);
}

/* 模式切换 */
.mode-switch {
  display: flex;
  gap: 0;
  border: 1px solid rgba(0, 180, 216, 0.3);
  border-radius: 6px;
  overflow: hidden;
}

.mode-btn {
  padding: 4px 14px;
  font-size: 12px;
  letter-spacing: 1px;
  border: none;
  background: var(--btn-bg);
  color: var(--btn-text);
  cursor: pointer;
  transition: all 0.15s ease;
}

.mode-btn.active {
  background: linear-gradient(180deg, #00b4d8, #0077b6);
  color: #ffffff;
  box-shadow: 0 0 10px rgba(0, 180, 216, 0.4);
}

.offline-map-btn {
  flex-shrink: 0;
  border-color: rgba(0, 180, 216, 0.3);
  color: var(--text-accent);
  background: var(--btn-bg);
}

.offline-map-btn:hover {
  background: var(--btn-hover-bg);
  border-color: rgba(0, 180, 216, 0.6);
}

@keyframes brand-breathe {
  0%,
  100% {
    opacity: 1;
    box-shadow: 0 0 10px rgba(0, 180, 216, 0.9);
  }
  50% {
    opacity: 0.5;
    box-shadow: 0 0 4px rgba(0, 180, 216, 0.4);
  }
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
  box-shadow: 0 0 8px rgba(255, 204, 0, 0.9);
  box-sizing: border-box;
}

.wp-diamond {
  width: 12px;
  height: 12px;
  background: #4aa3ff;
  border: 1px solid #fff;
  box-shadow: 0 0 6px rgba(74, 163, 255, 0.8);
  transform: rotate(45deg);
  box-sizing: border-box;
}

/* 深色地图底图下让 Leaflet 控件可见 */
.leaflet-container {
  background: #0a1428;
}

.leaflet-control-zoom a {
  background: var(--bg-card);
  color: var(--text-primary);
  border-color: var(--border-color);
}

.leaflet-control-zoom a:hover {
  background: var(--bg-hover);
}

.leaflet-control-attribution {
  background: rgba(7, 13, 26, 0.7);
  color: var(--text-dim);
}

.leaflet-control-attribution a {
  color: var(--text-accent);
}
</style>
