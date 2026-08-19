<!--
  显控中心（qgc Screen，DJI PC 地面站风格）

  以全屏 Cesium 3D 地图为主视角，整屏网格叠加六个面板（参考 DJI 地面站"人性化 3D 图形操作界面"）。
  布局为 WPF Grid 风格相对定位：三列按比例（22fr/62fr/16fr）分配，六面板各占一个
  grid-area，由父容器分配空间，任何分辨率都不会相互遮挡。
  面板为透明线框风格：背景全透明，地图完整可见；各面板内部网格化（1px 网格线），
  组件填充独立单元格且 max-width 兜底，绝不超出网格。

  ┌───────────┬─────────────────────────────────┬───────────┐
  │ 左上       │         顶中：状态条              │ 右上       │
  │ 仪表盘     │     Cesium 3D 全屏地图            │ 状态变量   │
  │ 姿态/速度  │ 飞机/轨迹/任务航线/返航点 H        │ 表格       │
  │ 高度/航向  │ 随点随行点击 + 视角控制按钮组      │ 变量名 | 值│
  │ 左下       │                                 │ 右下       │
  │ 飞行控制   │                                 │ 任务与航线 │
  │ 按钮组     │                                 │ 按钮组     │
  └───────────┴─────────────────────────────────┴───────────┘

  功能：
  1. 服务控制 + 遥测驱动（飞机 Billboard 按航向旋转、3D 轨迹、返航点 H）
  2. 飞行控制：解锁/起飞/降落/一键返航/任务开始/暂停/继续/模式
  3. 随点随行（点击地图即飞，SET_POSITION_TARGET_GLOBAL_INT）
  4. 键盘操控（WASD + 空格/Shift，SET_POSITION_TARGET_LOCAL_NED）
  5. 任务航线叠加（上传后显示，金色虚线）

   布局（参照 fj200c_main 主界面缩放自适应）：
   - screen-root 撑满剩余视口（导航栏下方），scaled-stage 以 1920×1080 设计尺寸
     做 CSS scale 缩放，任意分辨率下整体适配
   - Cesium 全屏地图置于 scaled-stage 之外（原生尺寸无 transform），拾取坐标天然正确，
     无需 2D 版那样的 getScale 补偿；scaled-stage 整层 pointer-events: none，
     点击穿透到地图，只有面板子元素拦截事件
   - 地图瓦片经后端代理加载（磁盘缓存，离线可用），token 经查询参数传递
-->
<template>
  <div ref="rootRef" class="screen-root">
    <!-- 全屏 Cesium 3D 地图（置于缩放舞台之外，保持原生尺寸，transform 不影响拾取） -->
    <div class="screen-map" ref="mapEl"></div>

    <div
      class="scaled-stage"
      :style="{
        width: DESIGN_W + 'px',
        height: DESIGN_H + 'px',
        transform: `scale(${scale.x}, ${scale.y})`,
      }"
    >
      <div class="qgc-screen-root">
        <!-- 整屏网格叠加层：六个面板各占一个 grid-area，互不遮挡干涉 -->
        <div class="screen-grid">

        <!-- 顶中：状态条（时钟 + 系统按钮，状态变量已并入右上表格） -->
        <div class="overlay top-bar area-top-bar">
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

        <!-- 左上：仪表盘（姿态仪 / 高度速度表 / 航向带纵向排列，点击标题栏收起/展开） -->
        <div class="overlay panel top-left area-top-left" :class="{ collapsed: instrCollapsed }">
          <div class="panel-title" @click="instrCollapsed = !instrCollapsed">
            <span class="title-text">飞行仪表盘</span>
            <span class="collapse-arrow">▾</span>
          </div>
          <div v-show="!instrCollapsed" class="instr-stack">
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

        <!-- 右上：状态变量表格（第一列变量名，第二列值，点击标题栏收起/展开） -->
        <div class="overlay panel top-right area-top-right" :class="{ collapsed: statCollapsed }">
          <div class="panel-title" @click="statCollapsed = !statCollapsed">
            <span class="title-text">状态变量</span>
            <span class="collapse-arrow">▾</span>
          </div>
          <div v-show="!statCollapsed" class="stat-table">
            <div class="st-row st-head">
              <span class="st-name">变量名</span>
              <span class="st-val">值</span>
            </div>
            <div class="st-row">
              <span class="st-name">服务状态</span>
              <span class="st-val" :class="serviceRunning ? 'ok' : 'dim'">{{ serviceRunning ? "运行中" : "已停止" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">遥测连接</span>
              <span class="st-val" :class="wsConnected ? 'ok' : 'pending'">{{ wsConnected ? "正常" : "连接中…" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">飞控连接</span>
              <span class="st-val" :class="telemetry.connected ? 'ok' : 'dim'">{{ telemetry.connected ? "已连接" : "未连接" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">解锁状态</span>
              <span class="st-val" :class="telemetry.armed ? 'warn' : 'dim'">{{ telemetry.armed ? "已解锁" : "未解锁" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">模式</span>
              <span class="st-val mode-val">{{ telemetry.mode || "—" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">GPS</span>
              <span class="st-val">{{ gpsText }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">卫星</span>
              <span class="st-val">{{ telemetry.satellites_visible ?? 0 }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">高度</span>
              <span class="st-val">{{ (telemetry.relative_alt ?? 0).toFixed(1) }} m</span>
            </div>
            <div class="st-row">
              <span class="st-name">地速</span>
              <span class="st-val">{{ (telemetry.groundspeed ?? 0).toFixed(1) }} m/s</span>
            </div>
            <div class="st-row">
              <span class="st-name">爬升率</span>
              <span class="st-val" :class="{ up: (telemetry.climb ?? 0) > 0.05, down: (telemetry.climb ?? 0) < -0.05 }">{{ (telemetry.climb ?? 0).toFixed(1) }} m/s</span>
            </div>
            <div class="st-row">
              <span class="st-name">距返航点</span>
              <span class="st-val">{{ ((telemetry.distance_home ?? 0) / 1000).toFixed(2) }} km</span>
            </div>
            <div class="st-row">
              <span class="st-name">返航方位</span>
              <span class="st-val">{{ (telemetry.bearing_home ?? 0).toFixed(0) }}°</span>
            </div>
            <div class="st-row">
              <span class="st-name">位置</span>
              <span class="st-val pos-val">{{ posText }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">帧率</span>
              <span class="st-val">{{ (telemetry.packet_rate ?? 0).toFixed(0) }} Hz</span>
            </div>
            <div class="st-row">
              <span class="st-name">数传</span>
              <span class="st-val">{{ rssiText }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">电池</span>
              <span class="st-val">{{ telemetry.battery_remaining ?? 0 }}%</span>
            </div>
            <div class="st-row">
              <span class="st-name">飞行时间</span>
              <span class="st-val">{{ flightTimeText }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">任务状态</span>
              <span class="st-val" :class="{ up: missionActive }">{{ missionStateText }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">当前航点</span>
              <span class="st-val">{{ missionCurrentSeq > 0 ? "#" + missionCurrentSeq : "—" }}</span>
            </div>
            <div class="st-row">
              <span class="st-name">航点数</span>
              <span class="st-val">{{ missionCount }}</span>
            </div>
          </div>
        </div>

        <!-- 左下：飞行控制按钮组（解锁/起飞/返航 + 起飞高度/模式/键盘操控，点击标题栏收起/展开） -->
        <div class="overlay panel bottom-left area-bottom-left" :class="{ collapsed: ctrlCollapsed }">
          <div class="panel-title" @click="ctrlCollapsed = !ctrlCollapsed">
            <span class="title-text">飞行控制</span>
            <span class="collapse-arrow">▾</span>
          </div>
          <div v-show="!ctrlCollapsed" class="ctrl-grid">
            <el-button size="small" class="g-arm" :loading="sending" :disabled="!telemetry.connected" @click="send('arm')">解锁</el-button>
            <el-button size="small" class="g-lock" :disabled="!telemetry.connected" @click="send('disarm')">锁定</el-button>
            <el-button size="small" class="g-takeoff" :loading="sending" :disabled="!telemetry.connected" @click="send('takeoff', takeoffAlt)">起飞</el-button>
            <el-button size="small" class="g-land" :disabled="!telemetry.connected" @click="send('land')">降落</el-button>
            <el-button size="small" class="g-rtl cmd-rtl" :disabled="!telemetry.connected" @click="send('rtl')">一键返航</el-button>
            <span class="g-alt-label ctrl-label">起飞高度</span>
            <el-input-number class="g-alt" v-model="takeoffAlt" :min="1" :max="200" size="small" controls-position="right" />
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

        <!-- 右下：任务控制按钮组（开始/暂停/继续 + 随点随行/上传/下载，点击标题栏收起/展开） -->
        <div class="overlay panel bottom-right area-bottom-right" :class="{ collapsed: missionCollapsed }">
          <div class="panel-title" @click="missionCollapsed = !missionCollapsed">
            <span class="title-text">任务与航线</span>
            <span class="collapse-arrow">▾</span>
          </div>
          <div v-show="!missionCollapsed" class="mission-grid">
            <el-button size="small" class="g-start cmd-start" :disabled="!telemetry.connected" @click="send('start')">开始执行</el-button>
            <el-button size="small" class="g-pause" :disabled="!telemetry.connected" @click="send('pause')">暂停</el-button>
            <el-button size="small" class="g-resume" :disabled="!telemetry.connected" @click="send('resume')">继续</el-button>
            <span class="g-click-label ctrl-label">随点随行</span>
            <el-switch class="g-click" v-model="clickToGo" size="small" :disabled="!telemetry.connected" />
            <span class="g-click-hint kbd-hint">点击地图即飞向目标</span>
            <span class="g-goto-label ctrl-label">目标高度</span>
            <el-input-number class="g-goto" v-model="gotoAlt" :min="5" :max="300" size="small" controls-position="right" />
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import * as Cesium from "cesium";
import "cesium/Build/Cesium/Widgets/widgets.css";
import { getSessionToken } from "@shared";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import { useWindowScale } from "@/qgc/composables/useWindowScale";
import AttitudeIndicator from "@/qgc/components/AttitudeIndicator.vue";
import HeadingTape from "@/qgc/components/HeadingTape.vue";
import AltitudeSpeedGauge from "@/qgc/components/AltitudeSpeedGauge.vue";
import OfflineMapPanel from "@/qgc/components/OfflineMapPanel.vue";

// Cesium 运行时资源（Workers/Assets）基址：dev 由 vite 中间件托管在 /cesium/，
// 构建产物拷贝在 dist/cesium/（经 /qgc 静态托管映射），统一按 BASE_URL 计算
(window as any).CESIUM_BASE_URL = `${import.meta.env.BASE_URL}cesium/`;

// ========== 分辨率缩放（1920×1080 设计稿，参照 fj200c_main 主界面） ==========

const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale();

// ========== 面板收起/展开（点击标题栏切换，收起后仅剩标题条，地图区域自动扩大） ==========

const instrCollapsed = ref(false);
const statCollapsed = ref(false);
const ctrlCollapsed = ref(false);
const missionCollapsed = ref(false);

// ========== 地图（Cesium 3D） ==========

const mapEl = ref<HTMLElement | null>(null);
let viewer: Cesium.Viewer | null = null;
let planeEntity: Cesium.Entity | null = null;
let trailEntity: Cesium.Entity | null = null;
let homeEntity: Cesium.Entity | null = null;
let missionLineEntity: Cesium.Entity | null = null;
let gotoEntity: Cesium.Entity | null = null;
const trailPositions: Cesium.Cartesian3[] = [];
const followPlane = ref(false);
let clickHandler: Cesium.ScreenSpaceEventHandler | null = null;
let mouseDown = false;
let mouseMoved = false;

/** 离线地图面板开关与初始中心 */
const offlinePanelVisible = ref(false);
const offlineCenter = ref<[number, number]>([31.2304, 121.4737]);

/** 打开离线地图面板（中心点同步为地图当前中心） */
function openOfflinePanel() {
  const c = viewer?.camera.positionCartographic;
  if (c) offlineCenter.value = [Cesium.Math.toDegrees(c.latitude), Cesium.Math.toDegrees(c.longitude)];
  offlinePanelVisible.value = true;
}

/** 飞机 SVG 图标（data URI，Billboard 按航向旋转） */
const PLANE_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40">` +
    `<circle cx="20" cy="20" r="17" fill="rgba(0,180,216,0.12)" stroke="rgba(0,180,216,0.5)" stroke-width="1"/>` +
    `<path d="M20 3 L21.5 12 L28 14.5 L28 16.5 L21.5 15.7 L21.5 22 L25.5 25 L25.5 27 L20 26.5 L14.5 27 L14.5 25 L18.5 22 L18.5 15.7 L12 16.5 L12 14.5 L18.5 12 Z" fill="#ffcc00" stroke="#0a1428" stroke-width="1.5"/>` +
    `<circle cx="20" cy="20" r="2.5" fill="#ffcc00"/></svg>`,
)}`;

/** 返航点 H 图标 */
const HOME_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="28" height="28"><circle cx="14" cy="14" r="13" fill="rgba(0,180,216,0.2)" stroke="#00b4d8" stroke-width="2"/><text x="14" y="18" font-family="Consolas,monospace" font-weight="bold" font-size="13" fill="#00b4d8" text-anchor="middle">H</text></svg>`,
)}`;

/** 随行目标图标（▶） */
const GOTO_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="11" fill="rgba(240,192,64,0.25)" stroke="#f0c040" stroke-width="2"/><text x="12" y="16" font-size="11" fill="#f0c040" text-anchor="middle">▶</text></svg>`,
)}`;

/** 更新飞机位置（Billboard + 3D 轨迹 + 跟随） */
function updatePlane(lat?: number, lon?: number, heading?: number, alt?: number) {
  if (viewer === null) return;
  // (0,0) 视为无效坐标（服务未启动时的默认遥测），回落到默认起飞点（上海），保证机体始终可见
  if (lat === undefined || lon === undefined || (lat === 0 && lon === 0)) {
    lat = 31.2304;
    lon = 121.4737;
  }
  const height = Math.max(alt ?? 0, 3);
  const position = new Cesium.ConstantPositionProperty(Cesium.Cartesian3.fromDegrees(lon, lat, height));
  if (planeEntity) {
    planeEntity.position = position;
    planeEntity.billboard!.rotation = new Cesium.ConstantProperty(-Cesium.Math.toRadians(heading ?? 0));
  } else {
    planeEntity = viewer.entities.add({
      position,
      billboard: {
        image: PLANE_SVG,
        width: 40,
        height: 40,
        rotation: -Cesium.Math.toRadians(heading ?? 0),
        alignedAxis: Cesium.Cartesian3.UNIT_Z,
        disableDepthTestDistance: Number.POSITIVE_INFINITY,
      },
    });
  }
  // 轨迹追加（与上一点距离 > 2 米才记录，避免静止堆积）
  const last = trailPositions[trailPositions.length - 1];
  const point = position.getValue(undefined)!;
  if (!last || Cesium.Cartesian3.distance(last, point) > 2) {
    trailPositions.push(point);
    if (trailEntity) {
      trailEntity.polyline!.positions = new Cesium.CallbackProperty(() => trailPositions, false);
    } else {
      trailEntity = viewer.entities.add({
        polyline: {
          positions: [...trailPositions],
          width: 2,
          material: new Cesium.ColorMaterialProperty(Cesium.Color.fromCssColorString("#ffcc00").withAlpha(0.85)),
          arcType: Cesium.ArcType.NONE,
        },
      });
    }
    if (trailPositions.length > 600) trailPositions.shift();
  }
}

/** 更新返航点 H */
function updateHome(homeLat?: number, homeLon?: number) {
  if (viewer === null || homeLat === undefined || homeLon === undefined || (homeLat === 0 && homeLon === 0)) return;
  const position = new Cesium.ConstantPositionProperty(Cesium.Cartesian3.fromDegrees(homeLon, homeLat, 5));
  if (homeEntity) {
    homeEntity.position = position;
  } else {
    homeEntity = viewer.entities.add({
      position,
      billboard: {
        image: HOME_SVG,
        width: 28,
        height: 28,
        disableDepthTestDistance: Number.POSITIVE_INFINITY,
      },
    });
  }
}

/** 更新任务航线（青色虚线，按航点海拔在 3D 空间连线） */
function updateMissionLine(items: { lat: number; lon: number; altitude?: number }[]) {
  if (viewer === null) return;
  if (items.length > 0) {
    const positions = items.map((i) => Cesium.Cartesian3.fromDegrees(i.lon, i.lat, Math.max(i.altitude ?? 3, 3)));
    if (missionLineEntity) {
      missionLineEntity.polyline!.positions = new Cesium.CallbackProperty(() => positions, false);
    } else {
      missionLineEntity = viewer.entities.add({
        polyline: {
          positions,
          width: 2,
          material: new Cesium.PolylineDashMaterialProperty({
            color: Cesium.Color.fromCssColorString("#00d4ff").withAlpha(0.8),
            dashLength: 16,
          }),
          arcType: Cesium.ArcType.NONE,
        },
      });
    }
  } else if (missionLineEntity) {
    viewer.entities.remove(missionLineEntity);
    missionLineEntity = null;
  }
}

/** 跟随飞机开关（trackedEntity 模式：相机自动跟随机体） */
function toggleFollow() {
  followPlane.value = !followPlane.value;
  if (!viewer) return;
  if (followPlane.value && planeEntity) {
    viewer.trackedEntity = planeEntity;
  } else {
    viewer.trackedEntity = undefined;
  }
}

/** 回中复位（有飞机则居中飞机，否则默认上海视野） */
function resetView() {
  if (!viewer) return;
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  const hasPlane = lat !== undefined && lat !== 0 && lon !== undefined;
  viewer.camera.flyTo({
    destination: Cesium.Cartesian3.fromDegrees(hasPlane ? lon! : 121.4737, hasPlane ? lat! : 31.2304, 2200),
    orientation: { heading: 0, pitch: Cesium.Math.toRadians(-55), roll: 0 },
    duration: 0.8,
  });
}

/** 缩放（按当前相机高度比例进退） */
function zoomBy(delta: number) {
  if (!viewer) return;
  const amount = Math.max(viewer.camera.positionCartographic.height * 0.55, 50);
  if (delta > 0) viewer.camera.zoomIn(amount);
  else viewer.camera.zoomOut(amount);
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
  // 目标点图标（浮在目标高度）
  const position = new Cesium.ConstantPositionProperty(
    Cesium.Cartesian3.fromDegrees(lon, lat, Math.max(gotoAlt.value, 3)),
  );
  if (gotoEntity) {
    gotoEntity.position = position;
  } else {
    gotoEntity = viewer!.entities.add({
      position,
      billboard: {
        image: GOTO_SVG,
        width: 24,
        height: 24,
        disableDepthTestDistance: Number.POSITIVE_INFINITY,
      },
    });
  }
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
    updatePlane(t.lat, t.lon, t.heading, t.relative_alt);
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
  // 初始化 Cesium 3D 地球（深色主题；瓦片经后端代理加载，磁盘缓存离线可用）
  const token = encodeURIComponent(getSessionToken() ?? "");
  viewer = new Cesium.Viewer(mapEl.value!, {
    baseLayer: new Cesium.ImageryLayer(
      new Cesium.UrlTemplateImageryProvider({
        url: `/api/qgc/tiles/{z}/{x}/{y}?token=${token}`,
        maximumLevel: 19,
      }),
    ),
    baseLayerPicker: false,
    geocoder: false,
    homeButton: false,
    sceneModePicker: false,
    navigationHelpButton: false,
    animation: false,
    timeline: false,
    fullscreenButton: false,
    infoBox: false,
    selectionIndicator: false,
    skyBox: false,
    skyAtmosphere: false,
  });
  viewer.scene.globe.baseColor = Cesium.Color.fromCssColorString("#0a1428");
  const cam = viewer.scene.screenSpaceCameraController;
  cam.minimumZoomDistance = 5;
  cam.maximumZoomDistance = 5000000;

  // 默认视野定位到上海（模拟器/默认任务区域），保证地图可见、机体立即可见
  viewer.camera.setView({
    destination: Cesium.Cartesian3.fromDegrees(121.4737, 31.2304, 2200),
    orientation: { heading: 0, pitch: Cesium.Math.toRadians(-55), roll: 0 },
  });

  // 随点随行点击（左键按下+移动视为相机拖拽，不触发点击）
  clickHandler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);
  clickHandler.setInputAction(() => {
    mouseDown = true;
    mouseMoved = false;
  }, Cesium.ScreenSpaceEventType.LEFT_DOWN);
  clickHandler.setInputAction(() => {
    if (mouseDown) mouseMoved = true;
  }, Cesium.ScreenSpaceEventType.MOUSE_MOVE);
  clickHandler.setInputAction((e: Cesium.ScreenSpaceEventHandler.PositionedEvent) => {
    if (mouseMoved || viewer === null) return;
    const cart = viewer.camera.pickEllipsoid(e.position, viewer.scene.globe.ellipsoid);
    if (!cart) return;
    const c = Cesium.Cartographic.fromCartesian(cart);
    onMapClick(Cesium.Math.toDegrees(c.latitude), Cesium.Math.toDegrees(c.longitude));
  }, Cesium.ScreenSpaceEventType.LEFT_UP);

  // 初始遥测快照（无有效遥测时机体落在默认起飞点，相机保持上海视野）
  try {
    const response = await qgcApi.getTelemetry();
    const t = response.data;
    if (t) {
      updatePlane(t.lat, t.lon, t.heading, t.relative_alt);
      if (t.lat !== 0 && t.lon !== 0) {
        viewer.camera.setView({
          destination: Cesium.Cartesian3.fromDegrees(t.lon, t.lat, 2200),
          orientation: { heading: 0, pitch: Cesium.Math.toRadians(-55), roll: 0 },
        });
      }
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
  clickHandler?.destroy();
  clickHandler = null;
  viewer?.destroy();
  viewer = null;
  planeEntity = null;
  trailEntity = null;
  homeEntity = null;
  missionLineEntity = null;
  gotoEntity = null;
  trailPositions.length = 0;
});
</script>

<style scoped>
/* ============ 缩放容器（参照 fj200c_main 主界面） ============ */

/* screen-root 撑满剩余视口（导航栏下方），scaled-stage 以 1920×1080 设计尺寸居中缩放 */
.screen-root {
  flex: 1;
  min-height: 0;
  width: 100%;
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--bg-page);
  overflow: hidden;
}

.scaled-stage {
  transform-origin: center center;
  overflow: hidden;
  flex-shrink: 0;
  position: relative;
  z-index: 1;
  /* 整层不拦截鼠标：点击穿透到下方的 Cesium 全屏地图，仅面板子元素拦截事件 */
  pointer-events: none;
}

/* 1920×1080 设计稿舞台：地图铺底 + 面板网格叠加 */
.qgc-screen-root {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
  pointer-events: none;
  /* ============ 网格化面板通用变量 ============ */
  /* 面板透明线框风格：容器底色只透出 1px 网格线，单元格全透明，地图完整可见 */
  --grid-line: rgba(56, 110, 170, 0.8);
  --cell-bg: transparent;
}

/* 全屏 Cesium 地图（置于缩放舞台之外，原生尺寸无 transform，拾取坐标天然正确） */
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
  /* 标题栏整体可点击：收起/展开面板 */
  cursor: pointer;
  user-select: none;
}

/* 标题文字占满剩余宽度，箭头靠右 */
.title-text {
  flex: 1;
}

/* 收起/展开箭头：展开时 ▾ 朝下，收起时旋转 90° 变 ▸ */
.collapse-arrow {
  font-size: 10px;
  color: var(--text-dim);
  transition: transform 0.2s ease;
}

.panel.collapsed .collapse-arrow {
  transform: rotate(-90deg);
}

/* 收起后仅剩标题条：去掉底部间距，保持面板边框 */
.panel.collapsed {
  padding-bottom: 8px;
}

.panel.collapsed .panel-title {
  margin-bottom: 0;
}

/* 面板透明后文字直接落在地图上，统一加深色描边保障可读性 */
.panel-title,
.st-name,
.ctrl-label,
.kbd-toggle-label,
.kbd-hint,
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

/* ============ 顶中状态条（时钟 + 系统按钮） ============ */

.top-bar {
  display: flex;
  align-items: center;
  width: max-content;
  padding: 6px 10px;
  border-radius: 8px;
  background: transparent;
  border: 1px solid var(--grid-line);
  white-space: nowrap;
}

.bar-side {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bar-time {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: var(--text-dim);
}

/* ============ 左上仪表盘（纵向排列） ============ */

/* 面板宽度不再固定：stretch 填满所在 grid 列（比例列宽随窗口缩放） */
.top-left {
  width: 100%;
}

/* 仪表纵向网格：姿态仪 / 高度速度表 / 航向带 自上而下，1px 网格线分隔，组件铺满单元格 */
.instr-stack {
  display: grid;
  grid-template-columns: 1fr;
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
  padding: 8px 10px;
}

/* 铺满列宽：去掉组件默认 max-width（组件根类名：attitude-indicator / gauge-row / heading-tape） */
.top-left :deep(.attitude-indicator) {
  max-width: none;
  width: 100%;
}

/* 限制仪表高度，避免面板过高挤压地图区域 */
.top-left :deep(.attitude-svg) {
  max-height: 160px;
}

/* 表盘限高后 ROLL/PITCH 绝对定位会压住角速率行，改为流内独立成行 */
.top-left :deep(.attitude-values) {
  position: static;
  padding: 6px 8px 0;
}

.top-left :deep(.arc-svg) {
  max-height: 130px;
}

.top-left :deep(.gauge-row) {
  max-width: none;
  width: 100%;
}

.top-left :deep(.heading-tape) {
  width: 100%;
}

/* 透明线框风格：组件内部深色底淡化，地图透出 */
.top-left :deep(.tape-window) {
  background: rgba(7, 13, 26, 0.5);
}

.top-left :deep(.rate-item),
.top-left :deep(.throttle-row) {
  background: rgba(7, 13, 26, 0.4);
  border-color: rgba(56, 110, 170, 0.5);
}

/* ============ 右上状态变量表格（第一列变量名，第二列值） ============ */

.top-right {
  width: 100%;
}

/* 两列表格：行间 1px 网格线由容器 gap 透出，列间网格线由行自身 gap 透出；
   首行为表头；过高时纵向滚动（舞台固定 1080px 高） */
.stat-table {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--grid-line);
  border: 1px solid var(--grid-line);
  border-radius: 6px;
  overflow-y: auto;
  overflow-x: hidden;
  max-height: 680px;
}

/* 行包装为显式 2 列网格（不用 display:contents，滚动容器内网格线渲染更可靠） */
.st-row {
  display: grid;
  grid-template-columns: 1fr 1.5fr;
  gap: 1px;
  background: var(--grid-line);
}

.st-name,
.st-val {
  min-width: 0;
  overflow: hidden;
  padding: 3px 8px;
  background: var(--cell-bg);
  align-self: stretch;
}

.st-name {
  font-size: 11px;
  letter-spacing: 1px;
  color: var(--text-dim);
  text-align: left;
}

.st-val {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: var(--text-primary);
  text-align: right;
  white-space: nowrap;
  text-overflow: ellipsis;
}

/* 表头行：底色加深以示区分 */
.st-head .st-name,
.st-head .st-val {
  background: rgba(10, 20, 40, 0.75);
  color: var(--text-accent);
  font-size: 10px;
  letter-spacing: 1.5px;
  padding: 4px 8px;
}

.st-val.ok {
  color: var(--text-success);
}

.st-val.dim {
  color: var(--text-dim);
}

.st-val.pending {
  color: var(--text-accent);
}

.st-val.up {
  color: var(--text-success);
}

.st-val.down {
  color: var(--text-danger);
}

.st-val.mode-val {
  color: var(--text-hex);
}

.st-val.pos-val {
  font-size: 11px;
  color: var(--text-accent-green);
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

/* ---- 飞行控制 4 列网格布局（相对网格位置） ---- */

.ctrl-grid {
  grid-template-columns: repeat(4, 1fr);
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
  grid-column: 1 / 3;
  grid-row: 2;
}

.g-alt-label {
  grid-column: 3 / 4;
  grid-row: 2;
}

.g-alt {
  grid-column: 4 / 5;
  grid-row: 2;
}

.g-mode-label {
  grid-column: 1 / 2;
  grid-row: 3;
}

.g-mode {
  grid-column: 2 / 5;
  grid-row: 3;
}

.g-kbd-label {
  grid-column: 1 / 2;
  grid-row: 4;
}

.g-kbd {
  grid-column: 2 / 3;
  grid-row: 4;
  justify-self: center;
}

.g-kbd-hint {
  grid-column: 3 / 5;
  grid-row: 4;
}

.g-ack {
  grid-column: 1 / -1;
  grid-row: 5;
}

/* ---- 任务控制 4 列网格布局（相对网格位置） ---- */

.mission-grid {
  grid-template-columns: repeat(4, 1fr);
}

.g-start {
  grid-column: 1 / 3;
  grid-row: 1;
}

.g-pause {
  grid-column: 3 / 4;
  grid-row: 1;
}

.g-resume {
  grid-column: 4 / 5;
  grid-row: 1;
}

.g-click-label {
  grid-column: 1 / 2;
  grid-row: 2;
}

.g-click {
  grid-column: 2 / 3;
  grid-row: 2;
  justify-self: center;
}

.g-click-hint {
  grid-column: 3 / 5;
  grid-row: 2;
}

.g-goto-label {
  grid-column: 1 / 2;
  grid-row: 3;
}

.g-goto {
  grid-column: 2 / 5;
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
/* Cesium 深色主题适配（非 scoped：覆盖 Viewer 注入的 DOM） */
.cesium-widget,
.cesium-widget canvas {
  width: 100%;
  height: 100%;
}

.cesium-viewer {
  background: #0a1428;
}

/* 深色底图下隐藏默认底栏（无数据来源署名时不显示） */
.cesium-viewer-bottom {
  background: rgba(7, 13, 26, 0.7);
}

.cesium-viewer-bottom .cesium-credit-textContainer a {
  color: var(--text-accent);
}
</style>
