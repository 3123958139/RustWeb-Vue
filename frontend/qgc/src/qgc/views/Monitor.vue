<!--
  飞控仪表盘（qgc Monitor）

  功能：
  1. 服务控制：启动/停止飞控通信服务（排他）
  2. 姿态仪 + 航向带 + 高度/速度表 + 电池计（10Hz WebSocket 遥测驱动）
  3. 命令按钮组：解锁/锁定/起飞/降落/返航 + 飞行模式下拉（command_ack 事件显示结果）
  4. 状态栏：连接状态、当前时间、错误信息
-->
<template>
  <div class="qgc-root">
    <!-- 工具栏 -->
    <div class="qgc-toolbar">
      <span class="toolbar-title">飞控仪表盘</span>
      <el-tag :type="serviceRunning ? 'success' : 'info'" size="small">
        {{ serviceRunning ? "服务运行中" : "服务已停止" }}
      </el-tag>
      <el-tag :type="wsConnected ? 'success' : 'warning'" size="small">
        {{ wsConnected ? "遥测连接正常" : "遥测连接中…" }}
      </el-tag>
      <el-tag :type="telemetry.connected ? 'success' : 'info'" size="small">
        {{ telemetry.connected ? "飞控已连接" : "飞控未连接" }}
      </el-tag>
      <el-tag v-if="telemetry.armed" type="danger" size="small">已解锁</el-tag>
      <el-tag v-else type="info" size="small">未解锁</el-tag>
      <div class="spacer"></div>
      <span class="qgc-clock">{{ currentTime }}</span>
      <el-button type="primary" size="small" :loading="starting || stopping" @click="onToggleService">
        {{ serviceRunning ? "停止服务" : "启动服务" }}
      </el-button>
    </div>

    <!-- 仪表区 -->
    <div class="qgc-instruments">
      <!-- 姿态仪 -->
      <div class="instrument-card">
        <div class="card-label">姿态</div>
        <AttitudeIndicator :roll="telemetry.roll ?? 0" :pitch="telemetry.pitch ?? 0" />
      </div>

      <!-- 航向 -->
      <div class="instrument-card">
        <div class="card-label">航向</div>
        <HeadingTape :heading="telemetry.heading ?? 0" />
      </div>

      <!-- 高度/速度 -->
      <div class="instrument-card">
        <div class="card-label">高度与速度</div>
        <AltitudeSpeedGauge
          :relative-alt="telemetry.relative_alt ?? 0"
          :groundspeed="telemetry.groundspeed ?? 0"
          :climb="telemetry.climb ?? 0"
        />
      </div>

      <!-- 电池 -->
      <div class="instrument-card">
        <div class="card-label">电池</div>
        <BatteryGauge :remaining="telemetry.battery_remaining ?? 0" :voltage="telemetry.voltage ?? 0" />
      </div>
    </div>

    <!-- 信息面板 + 命令区 -->
    <div class="qgc-bottom">
      <div class="info-panel qgc-panel">
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">模式</span>
            <span class="info-value qgc-state-tag">{{ telemetry.mode || "—" }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">GPS</span>
            <span class="info-value">{{ gpsText }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">卫星</span>
            <span class="info-value">{{ telemetry.satellites_visible ?? 0 }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">爬升率</span>
            <span class="info-value">{{ (telemetry.climb ?? 0).toFixed(1) }} m/s</span>
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
            <span class="info-label">帧率</span>
            <span class="info-value">{{ (telemetry.packet_rate ?? 0).toFixed(0) }} Hz</span>
          </div>
          <div class="info-item">
            <span class="info-label">sysid</span>
            <span class="info-value">{{ telemetry.sysid ?? 0 }}</span>
          </div>
        </div>
      </div>

      <!-- 命令区 -->
      <div class="command-panel qgc-panel">
        <div class="command-row qgc-command-buttons">
          <el-button size="small" class="cmd-arm" :loading="sending" :disabled="!telemetry.connected" @click="sendCommand('arm')">解锁</el-button>
          <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('disarm')">锁定</el-button>
          <el-button size="small" class="cmd-takeoff" :loading="sending" :disabled="!telemetry.connected" @click="sendCommand('takeoff', takeoffAlt)">起飞</el-button>
          <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('land')">降落</el-button>
          <el-button size="small" :disabled="!telemetry.connected" @click="sendCommand('rtl')">返航</el-button>
          <el-input-number v-model="takeoffAlt" :min="1" :max="200" size="small" controls-position="right" style="width: 100px" />
          <span class="takeoff-hint">起飞高度(m)</span>
        </div>
        <div class="command-row">
          <span class="mode-label">飞行模式</span>
          <el-select v-model="selectedMode" size="small" style="width: 160px" :disabled="!telemetry.connected" @change="onModeChange">
            <el-option v-for="m in copterModes" :key="m" :label="m.toUpperCase()" :value="m" />
          </el-select>
          <span v-if="lastAckText" class="ack-text">{{ lastAckText }}</span>
        </div>
      </div>
    </div>

    <!-- 底部状态栏 -->
    <div class="status-bar">
      <div class="status-bar-left">
        <span class="status-item">状态: {{ statusText }}</span>
        <span class="status-divider">|</span>
        <span class="status-item">遥测: {{ posText }}</span>
      </div>
      <div class="status-bar-right">
        <span v-if="errorMessage" class="status-item status-error">{{ errorMessage }}</span>
        <span class="status-item">{{ currentTime }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { qgcApi } from "@/api";
import { useQgcEvents } from "@/qgc/composables/useQgcEvents";
import AttitudeIndicator from "@/qgc/components/AttitudeIndicator.vue";
import HeadingTape from "@/qgc/components/HeadingTape.vue";
import BatteryGauge from "@/qgc/components/BatteryGauge.vue";
import AltitudeSpeedGauge from "@/qgc/components/AltitudeSpeedGauge.vue";

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
.qgc-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #17181b;
  padding: 16px;
  box-sizing: border-box;
  gap: 12px;
}

.qgc-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.toolbar-title {
  font-size: 18px;
  font-weight: 600;
  color: #e0e0e0;
}

.spacer { flex: 1; }

.qgc-clock {
  font-size: 13px;
  color: #909399;
  white-space: nowrap;
}

.qgc-instruments {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 12px;
}

.instrument-card {
  background-color: #1f2126;
  border: 1px solid #2c2f36;
  border-radius: 6px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.card-label {
  font-size: 13px;
  color: #909399;
}

.qgc-bottom {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.info-panel {
  flex: 1;
  min-width: 320px;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 8px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.info-label {
  font-size: 11px;
  color: #909399;
}

.info-value {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 15px;
  color: #e0e0e0;
}

.command-panel {
  flex: 1;
  min-width: 380px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.command-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.mode-label {
  font-size: 13px;
  color: #909399;
}

.takeoff-hint {
  font-size: 12px;
  color: #909399;
}

.ack-text {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
  color: #5cb85c;
}

.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 28px;
  padding: 0 12px;
  background-color: #2c2f36;
  color: #e0e0e0;
  font-size: 12px;
  border-radius: 4px;
  flex-shrink: 0;
}

.status-bar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-bar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-divider {
  color: #606266;
}

.status-error {
  color: #f56c6c;
}
</style>
