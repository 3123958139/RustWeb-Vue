<!--
  飞控仪表盘（qgc Monitor，HUD 主题）

  功能：
  1. 服务控制：启动/停止飞控通信服务（排他）
  2. 姿态仪 + 航向罗盘带 + 高度/速度弧线表 + 电池计（10Hz WebSocket 遥测驱动）
  3. 命令按钮组：解锁/锁定/起飞/降落/返航 + 飞行模式下拉（command_ack 事件显示结果）
  4. 状态栏：连接状态、当前时间、错误信息

  布局（参照 fj200c_main 主仪表盘）：
  - screen-root 撑满剩余视口（导航栏下方），scaled-stage 以 1920×1080 设计尺寸
    做 CSS scale 缩放，任意分辨率下整体等比适配
  - app-container 纵向三段：工具栏 / 主内容（仪表区 + 信息命令区）/ 底部状态栏
  - 状态栏固定在页面最底部（flex 尾端，不随内容漂移）
-->
<template>
  <div ref="rootRef" class="screen-root">
    <div
      class="scaled-stage"
      :style="{
        width: DESIGN_W + 'px',
        height: DESIGN_H + 'px',
        transform: `scale(${scale.x}, ${scale.y})`,
      }"
    >
      <div class="app-container">
        <!-- 工具栏 -->
        <header class="qgc-toolbar">
          <div class="toolbar-brand">
            <span class="brand-dot"></span>
            <span class="toolbar-title">飞控仪表盘</span>
          </div>
          <div class="status-chips">
            <span class="chip" :class="serviceRunning ? 'ok' : 'idle'"><i class="chip-dot"></i>{{ serviceRunning ? "服务运行中" : "服务已停止" }}</span>
            <span class="chip" :class="wsConnected ? 'ok' : 'pending'"><i class="chip-dot"></i>{{ wsConnected ? "遥测连接正常" : "遥测连接中…" }}</span>
            <span class="chip" :class="telemetry.connected ? 'ok' : 'idle'"><i class="chip-dot"></i>{{ telemetry.connected ? "飞控已连接" : "飞控未连接" }}</span>
            <span class="chip" :class="telemetry.armed ? 'warn' : 'idle'"><i class="chip-dot"></i>{{ telemetry.armed ? "已解锁" : "未解锁" }}</span>
            <span class="chip"><i class="chip-dot hz"></i>帧率 {{ (telemetry.packet_rate ?? 0).toFixed(0) }} Hz</span>
          </div>
          <div class="spacer"></div>
          <span class="qgc-clock">{{ currentTime }}</span>
          <el-button type="primary" size="small" class="qgc-service-btn" :loading="starting || stopping" @click="onToggleService">
            {{ serviceRunning ? "停止服务" : "启动服务" }}
          </el-button>
        </header>

        <!-- 主内容：仪表区（约 55% 高度）+ 信息/命令区 -->
        <main class="main-content">
          <section class="section-gauge">
            <!-- 姿态仪 -->
            <div class="instrument-card">
              <div class="card-label"><i class="label-dot cyan"></i>姿态</div>
              <div class="card-body">
                <AttitudeIndicator
                  :roll="telemetry.roll ?? 0"
                  :pitch="telemetry.pitch ?? 0"
                  :connected="telemetry.connected"
                  :roll-rate="telemetry.roll_rate ?? 0"
                  :pitch-rate="telemetry.pitch_rate ?? 0"
                  :yaw-rate="telemetry.yaw_rate ?? 0"
                />
              </div>
            </div>

            <!-- 航向 -->
            <div class="instrument-card">
              <div class="card-label"><i class="label-dot cyan"></i>航向</div>
              <div class="card-body">
                <HeadingTape :heading="telemetry.heading ?? 0" />
              </div>
            </div>

            <!-- 高度/速度 -->
            <div class="instrument-card">
              <div class="card-label"><i class="label-dot green"></i>高度与速度</div>
              <div class="card-body">
                <AltitudeSpeedGauge
                  :relative-alt="telemetry.relative_alt ?? 0"
                  :groundspeed="telemetry.groundspeed ?? 0"
                  :climb="telemetry.climb ?? 0"
                  :throttle="telemetry.throttle ?? 0"
                />
              </div>
            </div>

            <!-- 电池 -->
            <div class="instrument-card">
              <div class="card-label"><i class="label-dot amber"></i>电池</div>
              <div class="card-body">
                <BatteryGauge
                  :remaining="telemetry.battery_remaining ?? 0"
                  :voltage="telemetry.voltage ?? 0"
                  :current="telemetry.current ?? 0"
                  :consumed-mah="telemetry.battery_consumed_mah ?? 0"
                />
              </div>
            </div>
          </section>

          <section class="section-bottom">
            <!-- 信息面板 -->
            <div class="info-panel qgc-panel">
              <div class="panel-title">遥测数据</div>
              <div class="info-grid">
                <div class="info-item">
                  <span class="info-label">模式</span>
                  <span class="info-value qgc-state-tag mode-value">{{ telemetry.mode || "—" }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">GPS</span>
                  <span class="info-value">{{ gpsText }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">卫星</span>
                  <span class="info-value">{{ telemetry.satellites_visible ?? 0 }} 颗</span>
                </div>
                <div class="info-item">
                  <span class="info-label">绝对高度</span>
                  <span class="info-value">{{ (telemetry.altitude ?? 0).toFixed(1) }} m</span>
                </div>
                <div class="info-item">
                  <span class="info-label">相对高度</span>
                  <span class="info-value">{{ (telemetry.relative_alt ?? 0).toFixed(1) }} m</span>
                </div>
                <div class="info-item">
                  <span class="info-label">爬升率</span>
                  <span class="info-value" :class="{ 'up': (telemetry.climb ?? 0) > 0.05, 'down': (telemetry.climb ?? 0) < -0.05 }">{{ (telemetry.climb ?? 0).toFixed(1) }} m/s</span>
                </div>
                <div class="info-item">
                  <span class="info-label">空速</span>
                  <span class="info-value">{{ (telemetry.airspeed ?? 0).toFixed(1) }} m/s</span>
                </div>
                <div class="info-item">
                  <span class="info-label">地速</span>
                  <span class="info-value">{{ (telemetry.groundspeed ?? 0).toFixed(1) }} m/s</span>
                </div>
                <div class="info-item">
                  <span class="info-label">油门</span>
                  <span class="info-value throttle-value">{{ (telemetry.throttle ?? 0).toFixed(0) }} %</span>
                </div>
                <div class="info-item">
                  <span class="info-label">电流</span>
                  <span class="info-value">{{ (telemetry.current ?? 0).toFixed(1) }} A</span>
                </div>
                <div class="info-item">
                  <span class="info-label">心跳</span>
                  <span class="info-value">{{ telemetry.last_heartbeat_ms ?? 0 }} ms</span>
                </div>
                <div class="info-item">
                  <span class="info-label">帧率</span>
                  <span class="info-value">{{ (telemetry.packet_rate ?? 0).toFixed(1) }} Hz</span>
                </div>
              </div>
            </div>

            <!-- 系统状态面板 -->
            <div class="sys-panel qgc-panel">
              <div class="panel-title">系统状态</div>
              <div class="sys-grid">
                <div class="info-item">
                  <span class="info-label">CPU 负载</span>
                  <span class="info-value load-value">{{ (telemetry.cpu_load ?? 0).toFixed(0) }} %</span>
                </div>
                <div class="info-item">
                  <span class="info-label">定位精度</span>
                  <span class="info-value">{{ telemetry.gps_eph && telemetry.gps_eph > 0 ? telemetry.gps_eph.toFixed(2) + " m" : "—" }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">已耗电量</span>
                  <span class="info-value">{{ ((telemetry.battery_consumed_mah ?? 0) / 1000).toFixed(2) }} Ah</span>
                </div>
                <div class="info-item">
                  <span class="info-label">系统 ID</span>
                  <span class="info-value">{{ telemetry.sysid ?? 0 }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">机型</span>
                  <span class="info-value">{{ vehicleTypeText }}</span>
                </div>
                <div class="info-item">
                  <span class="info-label">链路</span>
                  <span class="info-value" :class="telemetry.connected ? 'up' : 'down'">{{ telemetry.connected ? "正常" : "中断" }}</span>
                </div>
              </div>
            </div>

            <!-- 命令区 -->
            <div class="command-panel qgc-panel">
              <div class="panel-title">飞控指令</div>
              <div class="command-row qgc-command-buttons">
                <el-button size="small" class="cmd-arm" :loading="sending" :disabled="!telemetry.connected" @click="sendCommand('arm')">解锁</el-button>
                <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('disarm')">锁定</el-button>
                <el-button size="small" class="cmd-takeoff" :loading="sending" :disabled="!telemetry.connected" @click="sendCommand('takeoff', takeoffAlt)">起飞</el-button>
                <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('land')">降落</el-button>
                <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('rtl')">返航</el-button>
                <el-input-number v-model="takeoffAlt" :min="1" :max="200" size="small" controls-position="right" style="width: 90px" />
                <span class="takeoff-hint">起飞高度(m)</span>
              </div>
              <div class="command-row">
                <span class="mode-label">飞行模式</span>
                <el-select v-model="selectedMode" size="small" style="width: 150px" :disabled="!telemetry.connected" @change="onModeChange">
                  <el-option v-for="m in copterModes" :key="m" :label="m.toUpperCase()" :value="m" />
                </el-select>
                <span v-if="lastAckText" class="ack-text">{{ lastAckText }}</span>
              </div>
            </div>
          </section>
        </main>

        <!-- 底部状态栏（固定于页面底部） -->
        <footer class="section-status">
          <div class="status-bar">
            <div class="status-bar-left">
              <span class="heartbeat" :class="{ live: wsConnected }"></span>
              <span class="status-item">状态: {{ statusText }}</span>
              <span class="status-divider">|</span>
              <span class="status-item">遥测: {{ posText }}</span>
            </div>
            <div class="status-bar-right">
              <span v-if="errorMessage" class="status-item status-error">{{ errorMessage }}</span>
              <span class="status-item">{{ currentTime }}</span>
            </div>
          </div>
        </footer>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import { useWindowScale } from "@/qgc/composables/useWindowScale";
import AttitudeIndicator from "@/qgc/components/AttitudeIndicator.vue";
import HeadingTape from "@/qgc/components/HeadingTape.vue";
import BatteryGauge from "@/qgc/components/BatteryGauge.vue";
import AltitudeSpeedGauge from "@/qgc/components/AltitudeSpeedGauge.vue";

// ========== 分辨率缩放（1920×1080 设计稿） ==========

const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale();

// ========== 服务控制 ==========

/** 服务是否正在运行 */
const serviceRunning = ref(false);
/** 启动操作进行中 */
const starting = ref(false);
/** 停止操作进行中 */
const stopping = ref(false);
/** 状态描述文字 */
const statusText = ref("系统就绪");
/** 错误信息 */
const errorMessage = ref("");
/** 命令发送中 */
const sending = ref(false);

/** 起飞高度（米） */
const takeoffAlt = ref(10);

/** ArduPilot Copter 常用模式 */
const copterModes = ["stabilize", "alt_hold", "loiter", "guided", "auto", "rtl", "land", "brake", "poshold"];

/** 当前选中模式（跟随遥测） */
const selectedMode = ref("stabilize");

/** 最近一次命令回执文字 */
const lastAckText = ref("");

/**
 * 加载服务运行状态
 */
async function loadServiceStatus() {
  try {
    const response = await qgcApi.getServiceStatus();
    serviceRunning.value = response.data?.running ?? false;
  } catch {
    // 忽略错误（路由守卫处理未登录场景）
  }
}

/**
 * 切换服务状态（启动/停止）
 */
async function onToggleService() {
  if (serviceRunning.value) {
    stopping.value = true;
    try {
      await qgcApi.stopService();
      serviceRunning.value = false;
      statusText.value = "服务已停止";
      ElMessage.success("服务已停止");
    } catch (e: any) {
      ElMessage.error(e?.response?.data?.message || "停止服务失败");
    } finally {
      stopping.value = false;
    }
  } else {
    starting.value = true;
    try {
      await qgcApi.startService();
      serviceRunning.value = true;
      statusText.value = "服务运行中…";
      ElMessage.success("服务已启动");
    } catch (e: any) {
      ElMessage.error(e?.response?.data?.message || "启动服务失败");
    } finally {
      starting.value = false;
    }
  }
}

// ========== 命令与模式 ==========

/**
 * 发送飞控命令（arm / disarm / takeoff / land / rtl）
 * 回执经 WebSocket command_ack 事件异步返回
 */
async function sendCommand(command: string, altitude?: number) {
  sending.value = true;
  try {
    const response = await qgcApi.sendCommand(command, altitude);
    if (!response.data) {
      ElMessage.error("命令发送失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "命令发送失败");
  } finally {
    sending.value = false;
  }
}

/** 切换飞行模式 */
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

// ========== WebSocket 事件流 ==========

/** 命令结果码名称 → 中文描述 */
const ackResultText: Record<string, string> = {
  ACCEPTED: "已接受",
  TEMPORARILY_REJECTED: "临时拒绝",
  DENIED: "拒绝",
  UNSUPPORTED: "不支持",
  FAILED: "失败",
  IN_PROGRESS: "进行中",
};

/** GPS 定位类型描述 */
const gpsText = computed(() => {
  const fix = telemetry.value.gps_fix_type ?? 0;
  switch (fix) {
    case 3:
      return `3D (${telemetry.value.satellites_visible ?? 0} 星)`;
    case 4:
      return "3D 差分";
    case 2:
      return "2D";
    default:
      return "无定位";
  }
});

/** 遥测位置文字 */
const posText = computed(() => {
  const lat = telemetry.value.lat;
  const lon = telemetry.value.lon;
  if (lat === undefined || lon === undefined) return "—";
  return `${lat.toFixed(6)}, ${lon.toFixed(6)}`;
});

/** 机型映射（MAV_TYPE） */
const vehicleTypeMap: Record<number, string> = {
  1: "固定翼",
  2: "四旋翼",
  3: "共轴直升机",
  4: "直升机",
  10: "多旋翼",
  12: "VTOL",
  13: "飞艇",
  15: "地面车",
  18: "水面艇",
};

/** 机型文字 */
const vehicleTypeText = computed(() => {
  const v = telemetry.value.vehicle_type ?? 0;
  return vehicleTypeMap[v] ?? `未知 (${v})`;
});

/** 事件流：遥测驱动仪表，command_ack 显示回执，mission_progress 交给地图页任务面板 */
const { connected: wsConnected, telemetry, connect, disconnect } = useQgcEvents({
  onCommandAck: (ack) => {
    const resultName = ack.result_name || "UNKNOWN";
    lastAckText.value = `回执: ${ackResultText[resultName] ?? resultName} (cmd=${ack.command})`;
    if (resultName === "ACCEPTED") {
      ElMessage.success(lastAckText.value);
    } else {
      ElMessage.warning(lastAckText.value);
    }
  },
});

// ========== 时钟 ==========

/** 当前时间字符串 */
const currentTime = ref(new Date().toLocaleString("zh-CN", { hour12: false }));
/** 时钟定时器 */
let timerInterval: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  await loadServiceStatus();
  connect();
  timerInterval = setInterval(() => {
    currentTime.value = new Date().toLocaleString("zh-CN", { hour12: false });
  }, 1000);
});

onUnmounted(() => {
  disconnect();
  if (timerInterval) {
    clearInterval(timerInterval);
  }
});
</script>

<style scoped>
/* ============ 缩放容器（参照 fj200c_main） ============ */

.screen-root {
  flex: 1;
  min-height: 0;
  width: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  background:
    radial-gradient(1100px 380px at 50% -10%, rgba(0, 180, 216, 0.09), transparent 60%),
    var(--bg-page);
  overflow: hidden;
}

.scaled-stage {
  transform-origin: center center;
  overflow: hidden;
  flex-shrink: 0;
}

.app-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
  box-sizing: border-box;
  overflow: hidden;
}

/* ============ 工具栏 ============ */

.qgc-toolbar {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-shrink: 0;
  height: 52px;
}

.toolbar-brand {
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

.toolbar-title {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 2px;
  background: linear-gradient(90deg, #00b4d8, #4d9fff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.status-chips {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: var(--text-dim);
  white-space: nowrap;
}

.chip .chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #4a6282;
}

.chip.ok {
  color: var(--text-success);
  border-color: rgba(0, 230, 118, 0.3);
}

.chip.ok .chip-dot {
  background: var(--text-success);
  box-shadow: 0 0 6px rgba(0, 230, 118, 0.9);
}

.chip.pending {
  color: #e8d48a;
  border-color: rgba(241, 196, 15, 0.3);
}

.chip.pending .chip-dot {
  background: #f1c40f;
  box-shadow: 0 0 6px rgba(241, 196, 15, 0.9);
  animation: chip-blink 1s ease-in-out infinite;
}

.chip.warn {
  color: #ff8a7a;
  border-color: rgba(255, 51, 85, 0.35);
}

.chip.warn .chip-dot {
  background: var(--text-danger);
  box-shadow: 0 0 6px rgba(255, 51, 85, 0.9);
  animation: chip-blink 1s ease-in-out infinite;
}

.spacer {
  flex: 1;
}

.qgc-clock {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 14px;
  color: var(--text-dim);
  white-space: nowrap;
  letter-spacing: 1px;
}

/* ============ 主内容 ============ */

.main-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 仪表区：约占主内容 55% 高度，4 列并排 */
.section-gauge {
  flex: 0 0 55%;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
}

.instrument-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.04);
  overflow: hidden;
}

/* 卡片顶部发光描边 */
.instrument-card::before {
  content: "";
  position: absolute;
  top: 0;
  left: 12px;
  right: 12px;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(0, 180, 216, 0.6), transparent);
}

.card-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  letter-spacing: 2px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.label-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.label-dot.cyan {
  background: var(--text-accent);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.9);
}

.label-dot.green {
  background: var(--text-success);
  box-shadow: 0 0 6px rgba(0, 230, 118, 0.9);
}

.label-dot.amber {
  background: #f1c40f;
  box-shadow: 0 0 6px rgba(241, 196, 15, 0.9);
}

/* 仪表内容：垂直居中于卡片剩余高度 */
.card-body {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ============ 底部信息 + 命令区 ============ */

.section-bottom {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 14px;
}

.panel-title {
  font-size: 13px;
  letter-spacing: 2px;
  color: var(--text-accent);
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.panel-title::before {
  content: "";
  width: 4px;
  height: 13px;
  border-radius: 2px;
  background: linear-gradient(180deg, #00d4ff, #0077b6);
  box-shadow: 0 0 6px rgba(0, 180, 216, 0.8);
}

.info-panel {
  flex: 1.3;
  min-width: 0;
}

.sys-panel {
  flex: 0.9;
  min-width: 0;
}

.sys-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 14px 20px;
}

.throttle-value {
  color: var(--text-hex);
  text-shadow: 0 0 8px rgba(240, 192, 64, 0.5);
}

.load-value {
  color: var(--text-accent-green);
  text-shadow: 0 0 8px rgba(0, 212, 170, 0.5);
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px 20px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.info-label {
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--text-dim);
}

.info-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 16px;
  color: var(--text-primary);
  text-shadow: 0 0 8px rgba(0, 180, 216, 0.2);
}

.info-value.up {
  color: var(--text-success);
  text-shadow: 0 0 8px rgba(0, 230, 118, 0.5);
}

.info-value.down {
  color: var(--text-danger);
  text-shadow: 0 0 8px rgba(255, 51, 85, 0.5);
}

.mode-value {
  color: var(--text-hex);
  text-shadow: 0 0 8px rgba(240, 192, 64, 0.5);
}

.command-panel {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.command-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.mode-label {
  font-size: 13px;
  letter-spacing: 1px;
  color: var(--text-dim);
}

.takeoff-hint {
  font-size: 12px;
  color: var(--text-dim);
}

.ack-text {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 13px;
  color: var(--text-accent-green);
  text-shadow: 0 0 6px rgba(0, 212, 170, 0.5);
  animation: ack-in 0.3s ease-out;
}

/* ============ 底部状态栏（固定于底部） ============ */

.section-status {
  flex-shrink: 0;
}

.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 34px;
  padding: 0 14px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  font-size: 12px;
  border-radius: 8px;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.3);
}

.status-bar-left,
.status-bar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-divider {
  color: var(--border-hover);
}

.status-error {
  color: var(--text-danger);
}

/* 心跳灯：WS 连接正常时呼吸闪烁 */
.heartbeat {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #4a6282;
}

.heartbeat.live {
  background: var(--text-success);
  box-shadow: 0 0 8px rgba(0, 230, 118, 0.9);
  animation: heart-pulse 1.6s ease-in-out infinite;
}

/* ============ 动效 ============ */

@keyframes brand-breathe {
  0%,
  100% {
    box-shadow: 0 0 6px rgba(0, 180, 216, 0.6);
  }

  50% {
    box-shadow: 0 0 14px rgba(0, 180, 216, 1);
  }
}

@keyframes chip-blink {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.4;
  }
}

@keyframes heart-pulse {
  0%,
  100% {
    transform: scale(1);
    opacity: 1;
  }

  50% {
    transform: scale(0.75);
    opacity: 0.6;
  }
}

@keyframes ack-in {
  from {
    opacity: 0;
    transform: translateY(4px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
