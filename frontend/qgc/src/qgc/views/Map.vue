<!--
  地图与任务（qgc Map，Cesium 3D）

  CesiumJS 3D 地球 + 后端瓦片代理（`/api/qgc/tiles/{z}/{x}/{y}`，磁盘缓存离线可用）：
  1. 飞机 Billboard（SVG 图标按航向旋转，按遥测海拔悬浮在 3D 空间）+ 历史轨迹线（10Hz 遥测驱动）
  2. 点击地球添加航点（加入 MissionPanel 列表）；随点随行模式下点击即飞向目标
  3. 航点/首页 Billboard + 标签同步显示，MissionPanel 管理增删/上传/下载/清除
  4. mission_progress 事件实时驱动面板进度
  5. 视角控制按钮组（跟随飞机 / 回中复位 / 缩放）+ 飞机 HUD 叠加条

  说明：地图页为交互式页面，不做 1920×1080 整屏缩放（transform 会破坏 Cesium
  鼠标拾取坐标换算），改用与仪表盘一致的主题配色与面板风格。
  Cesium 相机默认与 Globe 同步旋转（3D 视角），底图瓦片仍由后端代理提供。
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
import { onMounted, onUnmounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import * as Cesium from "cesium";
import "cesium/Build/Cesium/Widgets/widgets.css";
import { getSessionToken } from "@shared";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import MissionPanel from "@/qgc/components/MissionPanel.vue";
import OfflineMapPanel from "@/qgc/components/OfflineMapPanel.vue";

// Cesium 运行时资源（Workers/Assets）基址：dev 由 vite 中间件托管在 /cesium/，
// 构建产物拷贝在 dist/cesium/（经 /qgc 静态托管映射），统一按 BASE_URL 计算
(window as any).CESIUM_BASE_URL = `${import.meta.env.BASE_URL}cesium/`;

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
  const c = viewer?.camera.positionCartographic;
  if (c) mapCenter.value = [Cesium.Math.toDegrees(c.latitude), Cesium.Math.toDegrees(c.longitude)];
  offlinePanelVisible.value = true;
}

// ---------- 规划 / 随行模式与视角控制 ----------

/** 地图交互模式：plan 添加航点 / goto 随点随行 */
const planMode = ref<"plan" | "goto">("plan");
/** 跟随飞机开关 */
const followPlane = ref(false);
/** 随行目标高度（米） */
const gotoAlt = ref(30);

/** 跟随飞机（trackedEntity 模式：相机自动跟随机体） */
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

// ---------- Cesium 3D 地图 ----------

/** Cesium 3D 地球实例 */
let viewer: Cesium.Viewer | null = null;
/** 飞机 Billboard 实体 */
let planeEntity: Cesium.Entity | null = null;
/** 轨迹线实体 */
let trailEntity: Cesium.Entity | null = null;
/** 航点实体列表（首页 + 航点，直接挂到 viewer.entities） */
let waypointEntities: Cesium.Entity[] = [];
/** 历史轨迹点（含海拔，3D 空间点） */
const trailPositions: Cesium.Cartesian3[] = [];
/** 点击事件处理器 */
let clickHandler: Cesium.ScreenSpaceEventHandler | null = null;
/** 按下左键后是否发生了拖拽（拖拽相机不算点击） */
let mouseDown = false;
let mouseMoved = false;

/** 飞机 SVG 图标（data URI，Billboard 按航向旋转，正北 0°） */
const PLANE_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 36 36">` +
    `<path d="M18 2 L19.5 10 L28 13 L28 15.5 L19.5 14.5 L19.5 21 L24 24 L24 26.5 L18 25.5 L12 26.5 L12 24 L16.5 21 L16.5 14.5 L8 15.5 L8 13 L16.5 10 Z" fill="#ffcc00" stroke="#000" stroke-width="1"/>`,
)}`;

/** 首页图标（金色圆点） */
const HOME_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14"><circle cx="7" cy="7" r="6" fill="#ffcc00" stroke="#000" stroke-width="1.5"/></svg>`,
)}`;

/** 普通航点图标（蓝色菱形） */
const DIAMOND_SVG = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12"><rect x="1" y="1" width="10" height="10" fill="#4aa3ff" stroke="#fff" stroke-width="1" transform="rotate(45 6 6)"/></svg>`,
)}`;

/** 更新飞机位置与航向（按遥测海拔悬浮在 3D 空间） */
function updatePlane(lat?: number, lon?: number, heading?: number, alt?: number) {
  // (0,0) 视为无效坐标（服务未启动时的默认遥测），避免飞机画到几内亚湾
  if (viewer === null || lat === undefined || lon === undefined || (lat === 0 && lon === 0)) return;
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
        width: 36,
        height: 36,
        rotation: -Cesium.Math.toRadians(heading ?? 0),
        alignedAxis: Cesium.Cartesian3.UNIT_Z,
        disableDepthTestDistance: Number.POSITIVE_INFINITY,
      },
    });
  }
  // 轨迹追加（与上一点距离 > 2 米才记录，避免静止堆积）
  const last = trailPositions[trailPositions.length - 1];
  if (!last || Cesium.Cartesian3.distance(last, position.getValue(undefined)) > 2) {
    const point = position.getValue(undefined)!;
    trailPositions.push(point);
    if (trailEntity) {
      trailEntity.polyline!.positions = new Cesium.CallbackProperty(() => trailPositions, false);
    } else {
      trailEntity = viewer.entities.add({
        polyline: {
          positions: [...trailPositions],
          width: 2,
          material: new Cesium.ColorMaterialProperty(Cesium.Color.fromCssColorString("#ffcc00").withAlpha(0.8)),
          arcType: Cesium.ArcType.NONE,
        },
      });
    }
    if (trailPositions.length > 500) trailPositions.shift();
  }
}

/** 刷新航点实体（与任务面板列表同步） */
function refreshWaypoints() {
  const v = viewer;
  if (v === null || missionPanelRef.value === null) return;
  // 先移除上一轮航点实体，再按当前列表重建（直接挂到 viewer.entities，保证被渲染）
  waypointEntities.forEach((e) => v.entities.remove(e));
  waypointEntities = [];
  const items = missionPanelRef.value.items;
  items.forEach((item) => {
    waypointEntities.push(
      v.entities.add({
        position: new Cesium.ConstantPositionProperty(
          Cesium.Cartesian3.fromDegrees(item.lon, item.lat, Math.max(item.altitude, 3)),
        ),
        billboard: {
          image: DIAMOND_SVG,
          width: 12,
          height: 12,
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
        label: {
          text: `#${item.seq} 高 ${item.altitude}m`,
          font: "11px Consolas, monospace",
          fillColor: Cesium.Color.fromCssColorString("#a8d8ff"),
          showBackground: true,
          backgroundColor: Cesium.Color.fromCssColorString("#0a1428").withAlpha(0.65),
          backgroundPadding: new Cesium.Cartesian2(4, 2),
          pixelOffset: new Cesium.Cartesian2(0, -20),
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
      }),
    );
  });
  // 首页（当前位置）在下载任务后由后端返回，刷新遥测时若任务列表含 seq=0 同步绘制
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  if (lat !== undefined && lon !== undefined && (lat !== 0 || lon !== 0)) {
    waypointEntities.push(
      v.entities.add({
        position: new Cesium.ConstantPositionProperty(Cesium.Cartesian3.fromDegrees(lon, lat, 3)),
        billboard: {
          image: HOME_SVG,
          width: 14,
          height: 14,
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
        label: {
          text: "首页",
          font: "11px Consolas, monospace",
          fillColor: Cesium.Color.fromCssColorString("#ffd766"),
          showBackground: true,
          backgroundColor: Cesium.Color.fromCssColorString("#0a1428").withAlpha(0.65),
          backgroundPadding: new Cesium.Cartesian2(4, 2),
          pixelOffset: new Cesium.Cartesian2(0, -18),
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
      }),
    );
  }
}

// ========== WebSocket 事件流 ==========

/** 事件流：遥测驱动飞机移动，任务进度驱动面板 */
const { telemetry, connect, disconnect } = useQgcEvents({
  onTelemetry: (t) => {
    updatePlane(t.lat, t.lon, t.heading, t.relative_alt);
  },
  onMissionProgress: (p) => {
    missionPanelRef.value?.applyProgress(p.state, p.total, p.received, p.result);
    // 下载/上传完成后刷新航点实体
    if (p.state === "idle") {
      refreshWaypoints();
    }
  },
});

onMounted(async () => {
  // 初始化 Cesium 3D 地球（深色主题：关闭星空/大气，底图取后端瓦片代理）
  const token = encodeURIComponent(getSessionToken() ?? "");
  viewer = new Cesium.Viewer(mapEl.value!, {
    baseLayer: new Cesium.ImageryLayer(
      new Cesium.UrlTemplateImageryProvider({
        url: `/api/qgc/tiles/{z}/{x}/{y}?token=${token}`,
        maximumLevel: 19,
      }),
    ),
    // 关闭默认控件，保持与 2D 版一致的极简交互
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
    // 深色主题：关闭星空与大气（底图即地球，skyBox/skyAtmosphere 会破坏深色观感）
    skyBox: false,
    skyAtmosphere: false,
  });
  // 深色海洋/底图底色 + 相机距离限制
  viewer.scene.globe.baseColor = Cesium.Color.fromCssColorString("#0a1428");
  const cam = viewer.scene.screenSpaceCameraController;
  cam.minimumZoomDistance = 5;
  cam.maximumZoomDistance = 5000000;

  // 航点实体直接挂到 viewer.entities（见 refreshWaypoints），此处无需初始化集合
  // 默认视野定位到上海（模拟器/默认任务区域），保证地图可见、点击添加航点立即可见
  viewer.camera.setView({
    destination: Cesium.Cartesian3.fromDegrees(121.4737, 31.2304, 2200),
    orientation: { heading: 0, pitch: Cesium.Math.toRadians(-55), roll: 0 },
  });

  // 点击地球：规划模式添加航点 / 随行模式下达飞行指令
  // 左键按下+移动视为相机拖拽，不触发点击（pickEllipsoid 坐标经椭圆体求交）
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
    const lat = Cesium.Math.toDegrees(c.latitude);
    const lon = Cesium.Math.toDegrees(c.longitude);
    if (planMode.value === "goto") {
      onMapClick(lat, lon);
    } else {
      missionPanelRef.value?.addExternalItem(lat, lon);
      refreshWaypoints();
    }
  }, Cesium.ScreenSpaceEventType.LEFT_UP);

  // 初始遥测快照（未启动服务时保持默认视野）
  try {
    const response = await qgcApi.getTelemetry();
    const t = response.data;
    if (t && t.lat !== 0 && t.lon !== 0) {
      updatePlane(t.lat, t.lon, t.heading, t.relative_alt);
      viewer.camera.setView({
        destination: Cesium.Cartesian3.fromDegrees(t.lon, t.lat, 2200),
        orientation: { heading: 0, pitch: Cesium.Math.toRadians(-55), roll: 0 },
      });
    }
  } catch {
    // 忽略错误
  }

  connect();
  refreshWaypoints();
});

onUnmounted(() => {
  disconnect();
  clickHandler?.destroy();
  clickHandler = null;
  // 移除航点实体并销毁地球（viewer.destroy 会清理其余实体）
  waypointEntities.forEach((e) => viewer?.entities.remove(e));
  waypointEntities = [];
  viewer?.destroy();
  viewer = null;
  planeEntity = null;
  trailEntity = null;
  trailPositions.length = 0;
});

// 任务面板列表变化（增删/模板/导入/上传下载/地图点击）时同步刷新地图航点实体
watch(
  () => missionPanelRef.value?.items,
  () => refreshWaypoints(),
  { deep: true },
);
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
