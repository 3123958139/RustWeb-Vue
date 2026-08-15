<!--
  显控中心（qgc Screen，DJI PC 地面站风格）

  以全屏地图为主视角，四角叠加面板（参考 DJI 地面站"人性化 3D 图形操作界面"）：

  ┌───────────┬─────────────────────────────────┬───────────┐
  │ 左上       │         顶中：状态条              │ 右上       │
  │ 飞行状态卡 │     Leaflet 全屏地图              │ 仪表盘     │
  │           │ 飞机/轨迹/任务航线/返航点 H        │ 姿态+航向  │
  │ 左下       │ 随点随行点击 + 视角控制按钮组      │ 速度+高度  │
  │ 飞行控制   │                                 │ 弧线表     │
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

    <!-- 顶中：状态条 -->
    <div class="overlay top-bar">
      <span class="bar-chip" :class="serviceRunning ? 'ok' : 'idle'"><i class="chip-dot"></i>{{ serviceRunning ? "服务运行中" : "服务已停止" }}</span>
      <span class="bar-chip" :class="wsConnected ? 'ok' : 'pending'"><i class="chip-dot"></i>{{ wsConnected ? "遥测正常" : "遥测连接中…" }}</span>
      <span class="bar-chip" :class="telemetry.connected ? 'ok' : 'idle'"><i class="chip-dot"></i>{{ telemetry.connected ? "飞控已连接" : "飞控未连接" }}</span>
      <span class="bar-chip" :class="telemetry.armed ? 'warn' : 'idle'"><i class="chip-dot"></i>{{ telemetry.armed ? "已解锁" : "未解锁" }}</span>
      <span class="bar-chip"><i class="chip-dot hz"></i>帧率 {{ (telemetry.packet_rate ?? 0).toFixed(0) }} Hz</span>
      <span class="bar-chip"><i class="chip-dot rssi"></i>数传 {{ rssiText }}</span>
      <span class="bar-chip"><i class="chip-dot batt"></i>电池 {{ telemetry.battery_remaining ?? 0 }}%</span>
      <span class="bar-chip"><i class="chip-dot time"></i>飞行 {{ flightTimeText }}</span>
      <div class="spacer"></div>
      <span class="bar-time">{{ currentTime }}</span>
      <el-button size="small" class="offline-map-btn" @click="openOfflinePanel">离线地图</el-button>
      <el-button type="primary" size="small" class="qgc-service-btn" :loading="starting || stopping" @click="onToggleService">
        {{ serviceRunning ? "停止服务" : "启动服务" }}
      </el-button>
    </div>

    <!-- 离线地图面板（瓦片离线保存 / 加载管理） -->
    <el-dialog v-model="offlinePanelVisible" title="离线地图" width="560px" append-to-body class="offline-dialog">
      <OfflineMapPanel :center="offlineCenter" />
    </el-dialog>

    <!-- 左上：飞行状态卡 -->
    <div class="overlay panel top-left">
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
        <div class="state-item">
          <span class="state-label">位置</span>
          <span class="state-value pos-val">{{ posText }}</span>
        </div>
      </div>
    </div>

    <!-- 右上：仪表盘 -->
    <div class="overlay panel top-right">
      <div class="panel-title">飞行仪表盘</div>
      <div class="gauge-row">
        <AttitudeIndicator :roll="telemetry.roll ?? 0" :pitch="telemetry.pitch ?? 0" :connected="telemetry.connected" :roll-rate="telemetry.roll_rate ?? 0" :pitch-rate="telemetry.pitch_rate ?? 0" :yaw-rate="telemetry.yaw_rate ?? 0" />
      </div>
      <HeadingTape :heading="telemetry.heading ?? 0" />
      <AltitudeSpeedGauge :relative-alt="telemetry.relative_alt ?? 0" :groundspeed="telemetry.groundspeed ?? 0" :climb="telemetry.climb ?? 0" :throttle="telemetry.throttle ?? 0" />
    </div>

    <!-- 左下：飞行控制 -->
    <div class="overlay panel bottom-left">
      <div class="panel-title">飞行控制</div>
      <div class="ctrl-row">
        <el-button size="small" class="cmd-arm" :loading="sending" :disabled="!telemetry.connected" @click="send('arm')">解锁</el-button>
        <el-button size="small" :disabled="!telemetry.connected" @click="send('disarm')">锁定</el-button>
        <el-button size="small" class="cmd-takeoff" :loading="sending" :disabled="!telemetry.connected" @click="send('takeoff', takeoffAlt)">起飞</el-button>
        <el-button size="small" :disabled="!telemetry.connected" @click="send('land')">降落</el-button>
        <el-button size="small" class="cmd-rtl" :disabled="!telemetry.connected" @click="send('rtl')">一键返航</el-button>
        <el-input-number v-model="takeoffAlt" :min="1" :max="200" size="small" controls-position="right" style="width: 84px" />
      </div>
      <div class="ctrl-row">
        <span class="ctrl-label">任务</span>
        <el-button size="small" class="cmd-start" :disabled="!telemetry.connected" @click="send('start')">开始执行</el-button>
        <el-button size="small" :disabled="!telemetry.connected" @click="send('pause')">暂停</el-button>
        <el-button size="small" :disabled="!telemetry.connected" @click="send('resume')">继续</el-button>
      </div>
      <div class="ctrl-row">
        <span class="ctrl-label">模式</span>
        <el-select v-model="selectedMode" size="small" style="width: 130px" :disabled="!telemetry.connected" @change="onModeChange">
          <el-option v-for="m in copterModes" :key="m" :label="m.toUpperCase()" :value="m" />
        </el-select>
        <span class="kbd-toggle-label">键盘操控</span>
        <el-switch v-model="kbdEnabled" size="small" :disabled="!telemetry.connected" />
        <span class="kbd-hint">W/S 前后 · A/D 左右 · 空格上升 · Shift 下降</span>
      </div>
      <div v-if="lastAckText" class="ack-text">{{ lastAckText }}</div>
    </div>

    <!-- 右下：任务控制 -->
    <div class="overlay panel bottom-right">
      <div class="panel-title">任务与航线</div>
      <div class="ctrl-row">
        <span class="ctrl-label">随点随行</span>
        <el-switch v-model="clickToGo" size="small" :disabled="!telemetry.connected" />
        <span class="kbd-hint">开启后点击地图即飞向目标</span>
      </div>
      <div class="ctrl-row">
        <span class="ctrl-label">目标高度</span>
        <el-input-number v-model="gotoAlt" :min="5" :max="300" size="small" controls-position="right" style="width: 84px" />
      </div>
      <div class="mission-state">
        <div class="state-item">
          <span class="state-label">任务状态</span>
          <span class="state-value" :class="{ up: missionActive }">{{ missionStateText }}</span>
        </div>
        <div class="state-item">
          <span class="state-label">当前航点</span>
          <span class="state-value">{{ missionCurrentSeq > 0 ? "#" + missionCurrentSeq : "—" }}</span>
        </div>
        <div class="state-item">
          <span class="state-label">航点数</span>
          <span class="state-value">{{ missionCount }}</span>
        </div>
      </div>
      <div class="ctrl-row">
        <el-button size="small" :loading="uploading" :disabled="!telemetry.connected" @click="onUpload">上传任务</el-button>
        <el-button size="small" :loading="downloading" :disabled="!telemetry.connected" @click="onDownload">下载任务</el-button>
      </div>
    </div>

    <!-- 右侧：视角控制 -->
    <div class="overlay view-ctrl">
      <button class="vc-btn" :class="{ active: followPlane }" title="跟随飞机" @click="toggleFollow">◎</button>
      <button class="vc-btn" title="回中复位" @click="resetView">⌂</button>
      <button class="vc-btn" title="放大" @click="zoomBy(1)">+</button>
      <button class="vc-btn" title="缩小" @click="zoomBy(-1)">−</button>
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
}

.screen-map {
  position: absolute;
  inset: 0;
}

/* ============ 悬浮层通用 ============ */

.overlay {
  position: absolute;
  z-index: 1000;
}

.panel {
  background: rgba(10, 20, 40, 0.78);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 10px 12px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.05);
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

.panel-title::before {
  content: "";
  width: 4px;
  height: 12px;
  border-radius: 2px;
  background: linear-gradient(180deg, #00d4ff, #0077b6);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.8);
}

/* ============ 顶中状态条 ============ */

.top-bar {
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-radius: 8px;
  background: rgba(10, 20, 40, 0.75);
  border: 1px solid var(--border-color);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  max-width: calc(100% - 520px);
  min-width: 560px;
  white-space: nowrap;
}

.bar-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-primary);
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

.spacer {
  flex: 1;
}

.bar-time {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: var(--text-dim);
}

/* ============ 左上状态卡 ============ */

.top-left {
  top: 62px;
  left: 12px;
  width: 232px;
}

.state-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 12px;
}

.state-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
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
  top: 62px;
  right: 12px;
  width: 300px;
}

.gauge-row {
  display: flex;
  justify-content: center;
}

.top-right :deep(.attitude-root) {
  transform: scale(0.82);
  transform-origin: center top;
  margin-bottom: -24px;
}

.top-right :deep(.heading-root) {
  transform: scale(0.9);
  transform-origin: center top;
}

.top-right :deep(.altspd-root) {
  transform: scale(0.85);
  transform-origin: center top;
  margin-top: -10px;
}

/* ============ 左下飞行控制 ============ */

.bottom-left {
  bottom: 12px;
  left: 12px;
  width: 470px;
}

.ctrl-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  flex-wrap: wrap;
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

.ack-text {
  margin-top: 8px;
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

.mission-state {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 8px;
  margin-top: 10px;
  padding: 8px 10px;
  background: rgba(7, 13, 26, 0.6);
  border: 1px solid rgba(30, 58, 95, 0.6);
  border-radius: 6px;
}

/* ============ 视角控制按钮组 ============ */

.view-ctrl {
  top: 50%;
  right: 12px;
  transform: translateY(-50%);
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

@keyframes blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
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
