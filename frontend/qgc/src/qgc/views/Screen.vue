<!--
  显控中心（qgc Screen，DJI PC 地面站风格）

  以全屏地图为主视角，整屏网格叠加六个面板（参考 DJI 地面站"人性化 3D 图形操作界面"）。
  布局为 WPF Grid 风格相对定位：三列按比例（22fr/62fr/16fr）分配，六面板各占一个
  grid-area，由父容器分配空间，任何分辨率都不会相互遮挡。
  面板为透明线框风格：背景全透明，地图完整可见；各面板内部网格化（1px 网格线），
  组件填充独立单元格且 max-width 兜底，绝不超出网格。

  ┌───────────┬─────────────────────────────────┬───────────┐
  │ 左上       │         顶中：状态条              │ 右上       │
  │ 飞行状态卡 │     Leaflet 全屏地图              │ 仪表盘     │
  │           │ 飞机/轨迹/任务航线/返航点 H        │ 姿态/速度  │
  │ 左下       │ 随点随行点击 + 视角控制按钮组      │ 高度弧线表 │
  │ 飞行控制   │                                 │ 航向带     │
  └───────────┴─────────────────────────────────┴───────────┘

  功能：
  1. 服务控制 + 遥测驱动（飞机 Marker 按航向旋转、轨迹、返航点 H）
  2. 飞行控制：解锁/起飞/降落/一键返航/任务开始/暂停/继续/模式
  3. 随点随行（点击地图即飞，SET_POSITION_TARGET_GLOBAL_INT）
  4. 键盘操控（WASD + 空格/Shift，SET_POSITION_TARGET_LOCAL_NED）
  5. 任务航线叠加（上传后显示，金色虚线）
-->
<template>
  <div class="qgc-screen-root">
<!-- 全屏地图 -->
    <div class="screen-map" ref="mapEl"></div>

    <!-- 整屏网格叠加层：六个面板各占一个 grid-area，互不遮挡干涉 -->
    <div class="screen-grid">

    <!-- 顶中：状态条 -->
    <div class="overlay top-bar area-top-bar">
      <div class="bar-grid">
        <span class="bar-chip" :class="serviceRunning ? 'ok' : 'idle'"><i class="chip-dot"></i><span class="chip-text">{{ serviceRunning ? "服务运行中" : "服务已停止" }}</span></span>
        <span class="bar-chip" :class="wsConnected ? 'ok' : 'pending'"><i class="chip-dot"></i><span class="chip-text">{{ wsConnected ? "遥测正常" : "遥测连接中…" }}</span></span>
        <span class="bar-chip" :class="telemetry.connected ? 'ok' : 'idle'"><i class="chip-dot"></i><span class="chip-text">{{ telemetry.connected ? "飞控已连接" : "飞控未连接" }}</span></span>
        <span class="bar-chip" :class="telemetry.armed ? 'warn' : 'idle'"><i class="chip-dot"></i><span class="chip-text">{{ telemetry.armed ? "已解锁" : "未解锁" }}</span></span>
        <span class="bar-chip"><i class="chip-dot hz"></i><span class="chip-text">帧率 {{ (telemetry.packet_rate ?? 0).toFixed(0) }} Hz</span></span>
        <span class="bar-chip"><i class="chip-dot rssi"></i><span class="chip-text">数传 {{ rssiText }}</span></span>
        <span class="bar-chip"><i class="chip-dot batt"></i><span class="chip-text">电池 {{ telemetry.battery_remaining ?? 0 }}%</span></span>
        <span class="bar-chip"><i class="chip-dot time"></i><span class="chip-text">飞行 {{ flightTimeText }}</span></span>
      </div>
      <div class="bar-side">
        <span class="bar-time">{{ currentTime }}</span>
        <el-button size="small" class="offline-map-btn" @click="openOfflinePanel">离线地图</el-button>
        <el-button type="primary" size="small" class="qgc-service-btn" :loading="starting || stopping" @click="onToggleService">
          {{ serviceRunning ? "停止服务" : "启动服务" }}
        </el-button>
      </div>
    </div>

    <!-- 离线地图面板（瓦片离线保存 / 加载管理） -->
    <el-dialog v-model="offlinePanelVisible" title="离线地图" width="560px" append-to-body class="offline-dialog">
      <OfflineMapPanel :center="offlineCenter" />
    </el-dialog>

    <!-- 左上：飞行状态卡 -->
    <div class="overlay panel top-left area-top-left">
      <div class="panel-title">飞行状态</div>
      <div class="state-grid">
        <div class="state-item">
          <span class="state-label">模式</span>
          <span class="state-value mode-val">{{ telemetry.mode || "—" }}</span>
        </div>
        <div class="state-item">
          <span class="state-label">GPS</span>
          <span class="state-value">{{ gpsText }}</span>
        </div>
        <div class="state-item">
          <span class="state-label">卫星</span>
          <span class="state-value">{{ telemetry.satellites_visible ?? 0 }}</span>
        </div>
        <div class="state-item">
          <span class="state-label">高度</span>
          <span class="state-value">{{ (telemetry.relative_alt ?? 0).toFixed(1) }} m</span>
        </div>
        <div class="state-item">
          <span class="state-label">地速</span>
          <span class="state-value">{{ (telemetry.groundspeed ?? 0).toFixed(1) }} m/s</span>
        </div>
        <div class="state-item">
          <span class="state-label">爬升率</span>
          <span class="state-value" :class="{ up: (telemetry.climb ?? 0) > 0.05, down: (telemetry.climb ?? 0) < -0.05 }">{{ (telemetry.climb ?? 0).toFixed(1) }} m/s</span>
        </div>
        <div class="state-item">
          <span class="state-label">距返航点</span>
          <span class="state-value">{{ ((telemetry.distance_home ?? 0) / 1000).toFixed(2) }} km</span>
        </div>
        <div class="state-item">
          <span class="state-label">返航方位</span>
          <span class="state-value">{{ (telemetry.bearing_home ?? 0).toFixed(0) }}°</span>
        </div>
        <div class="state-item wide">
          <span class="state-label">位置</span>
          <span class="state-value pos-val">{{ posText }}</span>
        </div>
      </div>
    </div>

    <!-- 右上：仪表盘 -->
    <div class="overlay panel top-right area-top-right">
      <div class="panel-title">飞行仪表盘</div>
      <!-- 姿态仪 / 高度速度表 / 航向带各占一格，网格线分隔，组件铺满单元格 -->
      <div class="instr-grid">
        <div class="instr-cell">
          <AttitudeIndicator :roll="telemetry.roll ?? 0" :pitch="telemetry.pitch ?? 0" :connected="telemetry.connected" :roll-rate="telemetry.roll_rate ?? 0" :pitch-rate="telemetry.pitch_rate ?? 0" :yaw-rate="telemetry.yaw_rate ?? 0" />
        </div>
        <div class="instr-cell">
          <AltitudeSpeedGauge :relative-alt="telemetry.relative_alt ?? 0" :groundspeed="telemetry.groundspeed ?? 0" :climb="telemetry.climb ?? 0" :throttle="telemetry.throttle ?? 0" />
        </div>
        <div class="instr-cell instr-cell-wide">
          <HeadingTape :heading="telemetry.heading ?? 0" />
        </div>
      </div>
    </div>

    <!-- 左下：飞行控制（每个控件独立网格单元格，grid-column/row 相对位置定位） -->
    <div class="overlay panel bottom-left area-bottom-left">
      <div class="panel-title">飞行控制</div>
      <div class="ctrl-grid">
        <el-button size="small" class="g-arm" :loading="sending" :disabled="!telemetry.connected" @click="send('arm')">解锁</el-button>
        <el-button size="small" class="g-lock" :disabled="!telemetry.connected" @click="send('disarm')">锁定</el-button>
        <el-button size="small" class="g-takeoff" :loading="sending" :disabled="!telemetry.connected" @click="send('takeoff', takeoffAlt)">起飞</el-button>
        <el-button size="small" class="g-land" :disabled="!telemetry.connected" @click="send('land')">降落</el-button>
        <el-button size="small" class="g-rtl cmd-rtl" :disabled="!telemetry.connected" @click="send('rtl')">一键返航</el-button>
        <span class="g-alt-label ctrl-label">起飞高度</span>
        <el-input-number class="g-alt" v-model="takeoffAlt" :min="1" :max="200" size="small" controls-position="right" />
        <span class="g-task-label ctrl-label">任务</span>
        <el-button size="small" class="g-start cmd-start" :disabled="!telemetry.connected" @click="send('start')">开始执行</el-button>
        <el-button size="small" class="g-pause" :disabled="!telemetry.connected" @click="send('pause')">暂停</el-button>
        <el-button size="small" class="g-resume" :disabled="!telemetry.connected" @click="send('resume')">继续</el-button>
        <span class="g-mode-label ctrl-label">模式</span>
        <el-select class="g-mode" v-model="selectedMode" size="small" :disabled="!telemetry.connected" @change="onModeChange">
          <el-option v-for="m in copterModes" :key="m" :label="m.toUpperCase()" :value="m" />
        </el-select>
        <span class="g-kbd-label kbd-toggle-label">键盘操控</span>
        <el-switch class="g-kbd" v-model="kbdEnabled" size="small" :disabled="!telemetry.connected" />
        <span class="g-kbd-hint kbd-hint">WASD 平移 · 空格↑ · Shift↓</span>
        <div v-if="lastAckText" class="g-ack ack-cell">{{ lastAckText }}</div>
      </div>
    </div>

    <!-- 右下：任务控制（每个控件独立网格单元格 + 任务状态三列子网格） -->
    <div class="overlay panel bottom-right area-bottom-right">
      <div class="panel-title">任务与航线</div>
      <div class="mission-grid">
        <span class="g-click-label ctrl-label">随点随行</span>
        <el-switch class="g-click" v-model="clickToGo" size="small" :disabled="!telemetry.connected" />
        <span class="g-click-hint kbd-hint">点击地图即飞向目标</span>
        <span class="g-goto-label ctrl-label">目标高度</span>
        <el-input-number class="g-goto" v-model="gotoAlt" :min="5" :max="300" size="small" controls-position="right" />
        <div class="g-mission mission-state">
          <div class="ms-item">
            <span class="state-label">任务状态</span>
            <span class="state-value" :class="{ up: missionActive }">{{ missionStateText }}</span>
          </div>
          <div class="ms-item">
            <span class="state-label">当前航点</span>
            <span class="state-value">{{ missionCurrentSeq > 0 ? "#" + missionCurrentSeq : "—" }}</span>
          </div>
          <div class="ms-item">
            <span class="state-label">航点数</span>
            <span class="state-value">{{ missionCount }}</span>
          </div>
        </div>
        <el-button size="small" class="g-upload" :loading="uploading" :disabled="!telemetry.connected" @click="onUpload">上传任务</el-button>
        <el-button size="small" class="g-download" :loading="downloading" :disabled="!telemetry.connected" @click="onDownload">下载任务</el-button>
      </div>
    </div>

    <!-- 右侧：视角控制 -->
    <div class="overlay view-ctrl area-view-ctrl">
      <button class="vc-btn" :class="{ active: followPlane }" title="跟随飞机" @click="toggleFollow">◎</button>
      <button class="vc-btn" title="回中复位" @click="resetView">⌂</button>
      <button class="vc-btn" title="放大" @click="zoomBy(1)">+</button>
      <button class="vc-btn" title="缩小" @click="zoomBy(-1)">−</button>
    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import { getSessionToken } from "@shared";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import AttitudeIndicator from "@/qgc/components/AttitudeIndicator.vue";
import HeadingTape from "@/qgc/components/HeadingTape.vue";
import AltitudeSpeedGauge from "@/qgc/components/AltitudeSpeedGauge.vue";
import OfflineMapPanel from "@/qgc/components/OfflineMapPanel.vue";

// ========== 地图 ==========

const mapEl = ref<HTMLElement | null>(null);
let map: L.Map | null = null;
let planeMarker: L.Marker | null = null;
let trailLine: L.Polyline | null = null;
let homeMarker: L.Marker | null = null;
let missionLine: L.Polyline | null = null;
let gotoMarker: L.Marker | null = null;
const trailPoints: L.LatLngTuple[] = [];
const followPlane = ref(false);

/** 离线地图面板开关与初始中心 */
const offlinePanelVisible = ref(false);
const offlineCenter = ref<[number, number]>([31.2304, 121.4737]);

/** 打开离线地图面板（中心点同步为地图当前中心） */
function openOfflinePanel() {
  const c = map?.getCenter();
  if (c) offlineCenter.value = [c.lat, c.lng];
  offlinePanelVisible.value = true;
}

/** 飞机 SVG 图标（按航向旋转） */
function planeIcon(heading: number): L.DivIcon {
  return L.divIcon({
    className: "screen-plane-icon",
    html: `<svg width="40" height="40" viewBox="0 0 40 40" style="transform: rotate(${heading}deg)">
      <circle cx="20" cy="20" r="17" fill="rgba(0,180,216,0.12)" stroke="rgba(0,180,216,0.5)" stroke-width="1"/>
      <path d="M20 3 L24 21 L20 37 L16 21 Z" fill="#ffcc00" stroke="#0a1428" stroke-width="1.5"/>
      <circle cx="20" cy="20" r="2.5" fill="#ffcc00"/>
    </svg>`,
    iconSize: [40, 40],
    iconAnchor: [20, 20],
  });
}

/** 更新飞机位置（Marker + 轨迹 + 跟随） */
function updatePlane(lat?: number, lon?: number, heading?: number) {
  if (map === null || lat === undefined || lon === undefined) return;
  const position: L.LatLngTuple = [lat, lon];
  if (planeMarker) {
    planeMarker.setLatLng(position);
    planeMarker.setIcon(planeIcon(heading ?? 0));
  } else {
    planeMarker = L.marker(position, { icon: planeIcon(heading ?? 0) }).addTo(map);
  }
  const last = trailPoints[trailPoints.length - 1];
  if (!last || distanceMeters(last, position) > 2) {
    trailPoints.push(position);
    if (trailLine) {
      trailLine.setLatLngs(trailPoints);
    } else {
      trailLine = L.polyline(trailPoints, { color: "#ffcc00", weight: 2, opacity: 0.85 }).addTo(map);
    }
    if (trailPoints.length > 600) trailPoints.shift();
  }
  if (followPlane.value) {
    map.panTo(position);
  }
}

/** 更新返航点 H */
function updateHome(homeLat?: number, homeLon?: number) {
  if (map === null || homeLat === undefined || homeLon === undefined || (homeLat === 0 && homeLon === 0)) return;
  const position: L.LatLngTuple = [homeLat, homeLon];
  if (homeMarker) {
    homeMarker.setLatLng(position);
  } else {
    homeMarker = L.marker(position, {
      icon: L.divIcon({
        className: "screen-home-icon",
        html: `<div class="home-badge">H</div>`,
        iconSize: [28, 28],
        iconAnchor: [14, 14],
      }),
    }).addTo(map);
  }
}

/** 更新任务航线（金色虚线） */
function updateMissionLine(items: { lat: number; lon: number }[]) {
  if (map === null) return;
  const pts = items.map((i) => [i.lat, i.lon] as L.LatLngTuple);
  if (missionLine) {
    if (pts.length > 0) {
      missionLine.setLatLngs(pts);
    } else {
      missionLine.remove();
      missionLine = null;
    }
  } else if (pts.length > 0) {
    missionLine = L.polyline(pts, { color: "#00d4ff", weight: 2, opacity: 0.8, dashArray: "8 6" }).addTo(map);
  }
}

/** 两点间距离（米） */
function distanceMeters(a: L.LatLngTuple, b: L.LatLngTuple): number {
  const R = 6371000;
  const dLat = ((b[0] - a[0]) * Math.PI) / 180;
  const dLon = ((b[1] - a[1]) * Math.PI) / 180;
  const lat1 = (a[0] * Math.PI) / 180;
  const lat2 = (b[0] * Math.PI) / 180;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}

/** 跟随飞机开关 */
function toggleFollow() {
  followPlane.value = !followPlane.value;
  if (followPlane.value && telemetry.value.lat !== undefined && telemetry.value.lon !== undefined) {
    map?.panTo([telemetry.value.lat, telemetry.value.lon]);
  }
}

/** 回中复位（有飞机则居中飞机，否则默认上海视野） */
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

// ========== 服务 / 命令 ==========

const serviceRunning = ref(false);
const starting = ref(false);
const stopping = ref(false);
const sending = ref(false);
const takeoffAlt = ref(30);
const selectedMode = ref("loiter");
const lastAckText = ref("");
const copterModes = ["stabilize", "alt_hold", "loiter", "auto", "guided", "rtl"];

async function loadServiceStatus() {
  try {
    const response = await qgcApi.getServiceStatus();
    serviceRunning.value = response.data?.running ?? false;
  } catch {
    // 忽略
  }
}

async function onToggleService() {
  if (serviceRunning.value) {
    stopping.value = true;
    try {
      await qgcApi.stopService();
      serviceRunning.value = false;
      disconnect();
    } finally {
      stopping.value = false;
    }
  } else {
    starting.value = true;
    try {
      await qgcApi.startService();
      serviceRunning.value = true;
      connect();
      loadMission();
    } catch (e: any) {
      ElMessage.error(e?.response?.data?.message || "启动服务失败");
    } finally {
      starting.value = false;
    }
  }
}

/** 发送飞控命令 */
async function send(command: string, altitude?: number) {
  sending.value = true;
  try {
    const response = await qgcApi.sendCommand(command, altitude ?? null);
    if (!response.data) {
      ElMessage.error("命令发送失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "命令发送失败");
  } finally {
    sending.value = false;
  }
}

/** 切换模式 */
async function onModeChange(mode: string) {
  try {
    const response = await qgcApi.setMode(mode);
    if (!response.data) {
      ElMessage.error("模式切换失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "模式切换失败");
  }
}

// ========== 随点随行 ==========

const clickToGo = ref(false);
const gotoAlt = ref(30);

/** 随点随行：点击地图飞向目标 */
async function onMapClick(lat: number, lon: number) {
  if (!clickToGo.value) return;
  gotoMarker?.remove();
  gotoMarker = L.marker([lat, lon], {
    icon: L.divIcon({
      className: "screen-goto-icon",
      html: `<div class="goto-badge">▶</div>`,
      iconSize: [24, 24],
      iconAnchor: [12, 12],
    }),
  }).addTo(map!);
  try {
    const response = await qgcApi.sendCommand("click_to_go", null, [lat, lon, gotoAlt.value]);
    if (!response.data) {
      ElMessage.error("随点随行指令发送失败");
    } else {
      ElMessage.success(`已下达随点随行指令 (${lat.toFixed(5)}, ${lon.toFixed(5)}, ${gotoAlt.value}m)`);
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "随点随行失败");
  }
}

// ========== 键盘操控（WASD + 空格/Shift） ==========

const kbdEnabled = ref(false);
/** 按键集合（key → 机体速度 m/s） */
const kbdKeys = new Set<string>();
let kbdTimer: ReturnType<typeof setInterval> | null = null;

function onKeyDown(e: KeyboardEvent) {
  if (!kbdEnabled.value) return;
  const k = e.key.toLowerCase();
  if (["w", "a", "s", "d", " ", "shift"].includes(k)) {
    e.preventDefault();
    kbdKeys.add(k);
  }
}

function onKeyUp(e: KeyboardEvent) {
  const k = e.key.toLowerCase();
  kbdKeys.delete(k);
}

/** 周期发送速度指令（100ms，按住持续飞行） */
function kbdLoop() {
  if (!kbdEnabled.value) return;
  let vx = 0;
  let vy = 0;
  let vz = 0;
  if (kbdKeys.has("w")) vx += 3;
  if (kbdKeys.has("s")) vx -= 3;
  if (kbdKeys.has("a")) vy -= 3;
  if (kbdKeys.has("d")) vy += 3;
  if (kbdKeys.has(" ")) vz -= 2;
  if (kbdKeys.has("shift")) vz += 2;
  if (vx !== 0 || vy !== 0 || vz !== 0) {
    qgcApi.sendCommand("move", null, [vx, vy, vz]).catch(() => {});
  }
}

// ========== 任务 ==========

const missionState = ref("idle");
const missionTotal = ref(0);
const missionReceived = ref(0);
const missionCurrentSeq = ref(-1);
const missionResult = ref("");
const uploading = ref(false);
const downloading = ref(false);
const missionCount = ref(0);

/** 加载任务快照并叠加航线 */
async function loadMission() {
  try {
    const response = await qgcApi.getMission();
    const data = response.data;
    if (!data) return;
    missionState.value = data.state;
    missionTotal.value = data.items.length;
    missionCount.value = data.items.filter((i) => i.seq > 0).length;
    updateMissionLine(data.items);
  } catch {
    // 忽略
  }
}

const missionActive = computed(() => ["uploading", "downloading", "clearing"].includes(missionState.value));
const missionStateText = computed(() => {
  switch (missionState.value) {
    case "uploading":
      return `上传中 ${missionReceived.value}/${missionTotal.value}`;
    case "downloading":
      return `下载中 ${missionReceived.value}/${missionTotal.value}`;
    case "clearing":
      return "清除中…";
    default:
      return missionResult.value === "ok" ? "任务就绪" : "待命";
  }
});

async function onUpload() {
  uploading.value = true;
  try {
    const response = await qgcApi.uploadMission([]);
    if (response.data) {
      ElMessage.success("任务已提交上传（请在「地图与任务」页规划航点）");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "任务上传失败");
  } finally {
    uploading.value = false;
  }
}

async function onDownload() {
  downloading.value = true;
  try {
    const response = await qgcApi.downloadMission();
    if (!response.data) {
      ElMessage.error("任务下载提交失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "任务下载失败");
  } finally {
    downloading.value = false;
  }
}

// ========== 展示辅助 ==========

const currentTime = ref(new Date().toLocaleString("zh-CN", { hour12: false }));
let timerInterval: ReturnType<typeof setInterval> | null = null;

const gpsText = computed(() => {
  const fix = telemetry.value.gps_fix_type ?? 0;
  switch (fix) {
    case 3:
      return "3D";
    case 4:
      return "3D 差分";
    case 2:
      return "2D";
    default:
      return "无定位";
  }
});

const posText = computed(() => {
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  if (lat === undefined || lon === undefined || (lat === 0 && lon === 0)) return "—";
  return `${lat.toFixed(5)}, ${lon.toFixed(5)}`;
});

const rssiText = computed(() => {
  const r = telemetry.value.radio_rssi ?? 127;
  return r >= 127 ? "—" : `${r} dBm`;
});

const flightTimeText = computed(() => {
  const s = telemetry.value.flight_time_s ?? 0;
  const m = Math.floor(s / 60);
  const ss = Math.floor(s % 60);
  return `${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
});

// ========== 事件流 ==========

const { connected: wsConnected, telemetry, connect, disconnect } = useQgcEvents({
  onTelemetry: (t) => {
    updatePlane(t.lat, t.lon, t.heading);
    updateHome(t.home_lat, t.home_lon);
  },
  onMissionProgress: (p) => {
    missionState.value = p.state;
    missionTotal.value = p.total;
    missionReceived.value = p.received;
    missionCurrentSeq.value = p.current_seq;
    if (p.result && p.result !== "ok") {
      missionResult.value = p.result;
    } else if (p.state === "idle") {
      missionResult.value = "ok";
      loadMission();
    }
  },
  onCommandAck: (ack) => {
    const names: Record<string, string> = { ACCEPTED: "已接受", TEMPORARILY_REJECTED: "临时拒绝", DENIED: "拒绝", UNSUPPORTED: "不支持", FAILED: "失败", IN_PROGRESS: "进行中" };
    const name = ack.result_name || "UNKNOWN";
    lastAckText.value = `回执: ${names[name] ?? name} (cmd=${ack.command})`;
  },
});

onMounted(async () => {
  // 初始化地图
  map = L.map(mapEl.value!, {
    center: [31.2304, 121.4737],
    zoom: 15,
    zoomControl: false,
  });
  // 瓦片经后端代理加载（磁盘缓存，离线可用）；token 经查询参数传递（img 无法带 Bearer 头）
  L.tileLayer(`/api/qgc/tiles/{z}/{x}/{y}?token=${encodeURIComponent(getSessionToken() ?? "")}`, {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
    maxZoom: 19,
  }).addTo(map);

  // 随点随行点击
  map.on("click", (e: L.LeafletMouseEvent) => {
    onMapClick(e.latlng.lat, e.latlng.lng);
  });

  // 初始遥测快照
  try {
    const response = await qgcApi.getTelemetry();
    const t = response.data;
    if (t && t.lat !== 0 && t.lon !== 0) {
      map.setView([t.lat, t.lon], 15);
      updatePlane(t.lat, t.lon, t.heading);
    }
  } catch {
    // 忽略
  }

  await loadServiceStatus();
  if (serviceRunning.value) {
    connect();
    loadMission();
  }

  timerInterval = setInterval(() => {
    currentTime.value = new Date().toLocaleString("zh-CN", { hour12: false });
  }, 1000);

  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("keyup", onKeyUp);
  kbdTimer = setInterval(kbdLoop, 100);
});

onUnmounted(() => {
  disconnect();
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("keyup", onKeyUp);
  if (kbdTimer !== null) clearInterval(kbdTimer);
  if (timerInterval !== null) clearInterval(timerInterval);
  map?.remove();
  map = null;
  planeMarker = null;
  trailLine = null;
  homeMarker = null;
  missionLine = null;
  gotoMarker = null;
});
</script>

<style scoped>
.qgc-screen-root {
  flex: 1;
  min-height: 0;
  position: relative;
  overflow: hidden;
  background: var(--bg-page);
  /* ============ 网格化面板通用变量 ============ */
  /* 面板透明线框风格：容器底色只透出 1px 网格线，单元格全透明，地图完整可见 */
  --grid-line: rgba(56, 110, 170, 0.8);
  --cell-bg: transparent;
}

.screen-map {
  position: absolute;
  inset: 0;
  z-index: 0;
}

/* ============ 整屏网格布局（WPF Grid 风格相对定位） ============
   对应 WPF：Grid 行/列用星号（* 比例）与 Auto（内容），子元素经 Grid.Row/Column 定位。
   - 三列按比例分配（22fr / 62fr / 16fr），面板 stretch 填满所在列，尺寸随窗口缩放
   - 行 auto(顶面板) / 1fr(地图区) / auto(底面板)，高度由内容与剩余空间分配
   - 面板位置由 grid-area 相对定位，由父容器分配空间，任何分辨率都不会相互遮挡；
     容器穿透点击，地图（screen-map 全屏铺底）仍可操作 */
.screen-grid {
  position: absolute;
  inset: 0;
  z-index: 1000;
  display: grid;
  grid-template-columns: minmax(210px, 22fr) minmax(0, 62fr) minmax(190px, 16fr);
  grid-template-rows: auto 1fr auto;
  grid-template-areas:
    "top-left top-bar top-right"
    ".        map     view-ctrl"
    "bottom-left map  bottom-right";
  gap: 8px;
  padding: 12px;
  pointer-events: none;
}

.screen-grid > * {
  pointer-events: auto;
}

.area-top-bar {
  grid-area: top-bar;
  justify-self: center;
  align-self: start;
}

.area-top-left {
  grid-area: top-left;
  justify-self: start;
  align-self: start;
}

.area-top-right {
  grid-area: top-right;
  justify-self: end;
  align-self: start;
}

.area-view-ctrl {
  grid-area: view-ctrl;
  justify-self: end;
  align-self: center;
}

.area-bottom-left {
  grid-area: bottom-left;
  justify-self: start;
  align-self: end;
}

.area-bottom-right {
  grid-area: bottom-right;
  justify-self: end;
  align-self: end;
}

/* ============ 悬浮层通用 ============ */

/* 面板定位由 .screen-grid 的 grid-area 决定（相对网格位置），此处仅保留外观 */
.overlay {
  z-index: 1000;
}

.panel {
  background: transparent;
  border: 1px solid var(--grid-line);
  border-radius: 10px;
  padding: 10px 12px;
}

.panel-title {
  font-size: 12px;
  letter-spacing: 2px;
  color: var(--text-accent);
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
}

/* 面板透明后文字直接落在地图上，统一加深色描边保障可读性 */
.panel-title,
.state-label,
.ctrl-label,
.kbd-toggle-label,
.kbd-hint,
.bar-chip,
.bar-time {
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.85);
}

.panel-title::before {
  content: "";
  width: 4px;
  height: 12px;
  border-radius: 2px;
  background: linear-gradient(180deg, #00d4ff, #0077b6);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.8);
}

/* ============ 顶中状态条（网格化：等宽单元格 + 网格线） ============ */

.top-bar {
  display: flex;
  align-items: stretch;
  gap: 10px;
  width: max-content;
  padding: 8px 10px;
  border-radius: 8px;
  background: transparent;
  border: 1px solid var(--grid-line);
  white-space: nowrap;
}

/* 状态单元格网格：1px 间距透出网格线，单元格等宽填充 */
.bar-grid {
  flex: 1;
  min-width: 0;
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 1px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 6px;
  overflow: hidden;
}

.bar-chip {
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-primary);
  background: var(--cell-bg);
}

.chip-text {
  overflow: hidden;
  text-overflow: ellipsis;
}

.bar-side {
  display: flex;
  align-items: center;
  gap: 8px;
}

.chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-dim);
}

.chip-dot.hz {
  background: var(--text-accent);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.9);
}

.chip-dot.rssi {
  background: var(--text-success);
  box-shadow: 0 0 6px rgba(0, 230, 118, 0.8);
}

.chip-dot.batt {
  background: var(--text-hex);
  box-shadow: 0 0 6px rgba(240, 192, 64, 0.8);
}

.chip-dot.time {
  background: var(--text-accent-green);
  box-shadow: 0 0 6px rgba(0, 212, 170, 0.8);
}

.bar-chip.ok .chip-dot {
  background: var(--text-success);
  box-shadow: 0 0 6px rgba(0, 230, 118, 0.8);
}

.bar-chip.warn .chip-dot {
  background: var(--text-hex);
  box-shadow: 0 0 6px rgba(240, 192, 64, 0.8);
}

.bar-chip.pending .chip-dot {
  background: var(--text-accent);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.8);
  animation: blink 1.2s ease-in-out infinite;
}

.bar-time {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: var(--text-dim);
}

/* ============ 左上状态卡 ============ */

/* 面板宽度不再固定：stretch 填满所在 grid 列（比例列宽随窗口缩放） */
.top-left {
  width: 100%;
}

/* 状态网格：1px 间距透出网格线，每项独立单元格填充 */
.state-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 6px;
  overflow: hidden;
}

.state-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  padding: 6px 8px;
  background: var(--cell-bg);
}

.state-item.wide {
  grid-column: 1 / -1;
}

.state-label {
  font-size: 10px;
  letter-spacing: 1.5px;
  color: var(--text-dim);
}

.state-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 14px;
  color: var(--text-primary);
  text-shadow: 0 0 8px rgba(0, 180, 216, 0.2);
}

.state-value.up {
  color: var(--text-success);
}

.state-value.down {
  color: var(--text-danger);
}

.mode-val {
  color: var(--text-hex);
}

.pos-val {
  font-size: 11px;
  color: var(--text-accent-green);
}

/* ============ 右上仪表盘 ============ */

.top-right {
  width: 100%;
}

/* 仪表网格：姿态仪 + 高度/速度表两列并排，航向带跨整行；1px 网格线分隔，组件铺满单元格 */
.instr-grid {
  display: grid;
  grid-template-columns: 1fr 1.1fr;
  gap: 1px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 6px;
  overflow: hidden;
}

.instr-cell {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
  background: var(--cell-bg);
}

.instr-cell > * {
  max-width: 100%;
}

.instr-cell-wide {
  grid-column: 1 / -1;
  padding: 8px 10px;
}

/* 铺满列宽：去掉组件默认 max-width（组件根类名：attitude-indicator / gauge-row / heading-tape） */
.top-right :deep(.attitude-indicator) {
  max-width: none;
  width: 100%;
}

/* 限制仪表高度，避免面板过高挤压地图区域 */
.top-right :deep(.attitude-svg) {
  max-height: 150px;
}

/* 表盘限高后 ROLL/PITCH 绝对定位会压住角速率行，改为流内独立成行 */
.top-right :deep(.attitude-values) {
  position: static;
  padding: 6px 8px 0;
}

.top-right :deep(.arc-svg) {
  max-height: 130px;
}

.top-right :deep(.gauge-row) {
  max-width: none;
  width: 100%;
}

.top-right :deep(.heading-tape) {
  width: 100%;
}

/* 透明线框风格：组件内部深色底淡化，地图透出 */
.top-right :deep(.tape-window) {
  background: rgba(7, 13, 26, 0.5);
}

.top-right :deep(.rate-item),
.top-right :deep(.throttle-row) {
  background: rgba(7, 13, 26, 0.4);
  border-color: rgba(56, 110, 170, 0.5);
}

/* ============ 左下飞行控制 / 右下任务控制：组件独立网格 ============ */

.bottom-left {
  width: 100%;
}

.bottom-right {
  width: 100%;
}

/* 组件网格容器：1px 间距透出网格线，每个控件独占一个相对网格位置 */
.ctrl-grid,
.mission-grid {
  display: grid;
  gap: 1px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 6px;
  overflow: hidden;
}

.ctrl-grid > *,
.mission-grid > * {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  background: var(--cell-bg);
  align-self: center;
}

/* 按钮 / 数字输入 / 下拉选择填充所在单元格；按钮文字单行不换行，窄列时截断而非穿格 */
.ctrl-grid :deep(.el-button),
.mission-grid :deep(.el-button) {
  width: 100%;
  margin: 0;
  padding: 5px 1px;
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
}

.ctrl-grid :deep(.el-input-number),
.mission-grid :deep(.el-input-number),
.ctrl-grid :deep(.el-select),
.mission-grid :deep(.el-select) {
  width: 100%;
}

/* 标签 / 提示在格内水平居中 */
.ctrl-grid .ctrl-label,
.mission-grid .ctrl-label {
  justify-self: center;
  text-align: center;
  line-height: 1.4;
}

.ctrl-grid .kbd-hint,
.mission-grid .kbd-hint {
  justify-self: start;
  padding: 0 4px;
  line-height: 1.4;
  white-space: nowrap;
}

/* ---- 飞行控制 8 列网格布局（相对网格位置） ---- */

.ctrl-grid {
  grid-template-columns: repeat(8, 1fr);
}

.g-arm {
  grid-column: 1 / 2;
  grid-row: 1;
}

.g-lock {
  grid-column: 2 / 3;
  grid-row: 1;
}

.g-takeoff {
  grid-column: 3 / 4;
  grid-row: 1;
}

.g-land {
  grid-column: 4 / 5;
  grid-row: 1;
}

.g-rtl {
  grid-column: 5 / 6;
  grid-row: 1;
}

.g-alt-label {
  grid-column: 6 / 7;
  grid-row: 1;
}

.g-alt {
  grid-column: 7 / 9;
  grid-row: 1;
}

.g-task-label {
  grid-column: 1 / 2;
  grid-row: 2;
}

.g-start {
  grid-column: 2 / 4;
  grid-row: 2;
}

.g-pause {
  grid-column: 4 / 6;
  grid-row: 2;
}

.g-resume {
  grid-column: 6 / 8;
  grid-row: 2;
}

.g-mode-label {
  grid-column: 1 / 2;
  grid-row: 3;
}

.g-mode {
  grid-column: 2 / 4;
  grid-row: 3;
}

.g-kbd-label {
  grid-column: 4 / 5;
  grid-row: 3;
}

.g-kbd {
  grid-column: 5 / 6;
  grid-row: 3;
}

.g-kbd-hint {
  grid-column: 6 / 9;
  grid-row: 3;
}

.g-ack {
  grid-column: 1 / -1;
  grid-row: 4;
}

/* ---- 任务控制 4 列网格布局（相对网格位置） ---- */

.mission-grid {
  grid-template-columns: repeat(4, 1fr);
}

.g-click-label {
  grid-column: 1 / 2;
  grid-row: 1;
}

.g-click {
  grid-column: 2 / 3;
  grid-row: 1;
  justify-self: center;
}

.g-click-hint {
  grid-column: 3 / 5;
  grid-row: 1;
}

.g-goto-label {
  grid-column: 1 / 2;
  grid-row: 2;
}

.g-goto {
  grid-column: 2 / 4;
  grid-row: 2;
}

.g-mission {
  grid-column: 1 / -1;
  grid-row: 3;
}

.g-upload {
  grid-column: 1 / 3;
  grid-row: 4;
}

.g-download {
  grid-column: 3 / 5;
  grid-row: 4;
}

.ctrl-label {
  font-size: 12px;
  letter-spacing: 1.5px;
  color: var(--text-dim);
}

.cmd-rtl {
  background: linear-gradient(180deg, #ff6a5e, #d63a2e) !important;
  border-color: #ff6a5e !important;
  color: #ffffff !important;
  font-weight: 700;
  box-shadow: 0 2px 12px rgba(230, 60, 50, 0.35);
}

.cmd-start {
  background: linear-gradient(180deg, #3ddc8f, #1f9d5c) !important;
  border-color: #3ddc8f !important;
  color: #ffffff !important;
  font-weight: 600;
  box-shadow: 0 2px 12px rgba(40, 200, 120, 0.35);
}

.kbd-toggle-label {
  font-size: 12px;
  color: var(--text-dim);
  margin-left: 8px;
}

.kbd-hint {
  font-size: 11px;
  color: var(--text-dim);
  letter-spacing: 0.5px;
}

.ack-cell {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: var(--text-accent-green);
  text-shadow: 0 0 6px rgba(0, 212, 170, 0.5);
}

/* ============ 右下任务控制 ============ */

.bottom-right {
  bottom: 12px;
  right: 12px;
  width: 300px;
}

/* 任务状态三列子网格（作为网格项嵌入，1px 网格线分隔） */
.mission-state {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 1px;
  background: var(--grid-line);
  border-radius: 4px;
  overflow: hidden;
  padding: 0;
}

.ms-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  padding: 6px 8px;
  background: var(--cell-bg);
}

/* ============ 视角控制按钮组 ============ */

.view-ctrl {
  display: flex;
  flex-direction: column;
  gap: 1px;
  width: max-content;
  padding: 4px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 8px;
}

.vc-btn {
  width: 30px;
  height: 30px;
  border-radius: 6px;
  border: none;
  background: var(--cell-bg);
  color: var(--btn-text);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s ease;
}

.vc-btn:hover {
  background: var(--btn-hover-bg);
  box-shadow: 0 0 10px rgba(0, 180, 216, 0.3);
}

.vc-btn.active {
  background: linear-gradient(180deg, #00b4d8, #0077b6);
  color: #ffffff;
  box-shadow: 0 0 12px rgba(0, 180, 216, 0.5);
}

@keyframes blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
  }
}

/* ============ 小屏兜底：加宽左右列比例、按钮字再缩小，组件始终不出格 ============ */
@media (max-width: 1599px) {
  .screen-grid {
    grid-template-columns: minmax(240px, 28fr) minmax(0, 54fr) minmax(180px, 18fr);
  }

  .ctrl-grid :deep(.el-button),
  .mission-grid :deep(.el-button) {
    font-size: 10px;
    padding: 5px 0;
  }

  .ctrl-grid .kbd-hint,
  .mission-grid .kbd-hint {
    font-size: 10px;
  }
}
</style>

<style>
/* Leaflet 覆盖层（非 scoped：divIcon 是字符串 HTML） */
.screen-plane-icon {
  background: transparent;
  border: none;
}

.screen-home-icon {
  background: transparent;
  border: none;
}

.home-badge {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: rgba(0, 180, 216, 0.2);
  border: 2px solid var(--text-accent, #00b4d8);
  color: var(--text-accent, #00b4d8);
  font-family: Consolas, monospace;
  font-weight: 700;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 12px rgba(0, 180, 216, 0.6);
}

.screen-goto-icon {
  background: transparent;
  border: none;
}

.goto-badge {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: rgba(240, 192, 64, 0.25);
  border: 2px solid var(--text-hex, #f0c040);
  color: var(--text-hex, #f0c040);
  font-size: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 10px rgba(240, 192, 64, 0.7);
}

.leaflet-container {
  background: #0a1428;
}

.leaflet-control-attribution {
  background: rgba(7, 13, 26, 0.7);
  color: var(--text-dim, #7d94b5);
}

.leaflet-control-attribution a {
  color: var(--text-accent, #00b4d8);
}
</style>
