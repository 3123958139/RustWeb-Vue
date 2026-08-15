<!--
  任务面板（MissionPanel）

  航点规划列表：增删/排序/上传/下载/清除。
  上传自动补首页（seq=0），任务协议进度经 WebSocket mission_progress 事件实时显示。

  扩展（参考 DJI 地面站航点规划）：
  - 航点属性：停留时间 / 转弯模式（定点、协调、自适应）/ 动作（无、拍照）
  - 航线模板：矩形扫描（区域内蛇形往返，间距可设）
  - 相对坐标编辑器：角度 + 距离生成航点
  - 任务导入 / 导出（JSON 文件）
-->
<template>
  <div class="mission-panel">
    <div class="panel-header">
      <span class="panel-title">
        <i class="title-dot"></i>
        任务规划
      </span>
      <el-tag :type="stateTagType" size="small" class="qgc-state-tag">{{ stateText }}</el-tag>
    </div>

    <!-- 进度条（非 idle 状态显示） -->
    <el-progress
      v-if="progressState !== 'idle' && missionTotal > 0"
      :percentage="progressPct"
      :status="progressState === 'clearing' ? undefined : 'success'"
      :stroke-width="10"
      class="mission-progress"
    />

    <!-- 工具行：模板 / 相对坐标 / 导入导出 -->
    <div class="tool-row">
          <el-button size="small" @click="applyTemplate">航线模板</el-button>
      <span class="tool-hint">矩形扫描</span>
      <el-input-number v-model="templateSpacing" :min="20" :max="500" size="small" controls-position="right" style="width: 84px" title="扫描间距(米)" />
      <span class="tool-hint">间距(m)</span>
      <el-divider direction="vertical" />
      <el-button size="small" @click="showPolar = !showPolar">相对坐标</el-button>
      <el-divider direction="vertical" />
      <el-button size="small" @click="exportMission">导出</el-button>
      <el-button size="small" @click="importMission">导入</el-button>
      <input ref="fileInput" type="file" accept=".json" style="display: none" @change="onFileSelected" />
    </div>

    <!-- 相对坐标编辑器（角度 + 距离生成航点） -->
    <div v-if="showPolar" class="polar-editor">
      <span class="polar-label">角度°</span>
      <el-input-number v-model="polarAngle" :min="0" :max="359" size="small" controls-position="right" style="width: 84px" />
      <span class="polar-label">距离(m)</span>
      <el-input-number v-model="polarDist" :min="5" :max="2000" size="small" controls-position="right" style="width: 84px" />
      <el-button size="small" type="primary" @click="addByPolar">生成航点</el-button>
      <span class="tool-hint">以最后航点为基准</span>
    </div>

    <el-table :data="items" size="small" max-height="210" empty-text="暂无航点（可点击地图添加或手动添加）" class="wp-table">
      <el-table-column prop="seq" label="#" width="40" />
      <el-table-column label="位置" min-width="150">
        <template #default="{ row }">{{ row.lat.toFixed(5) }}, {{ row.lon.toFixed(5) }}</template>
      </el-table-column>
      <el-table-column prop="altitude" label="高(m)" width="62" />
      <el-table-column label="停(s)" width="66">
        <template #default="{ $index }">
          <el-input-number v-model="items[$index].hold_time" :min="0" :max="300" size="small" controls-position="right" style="width: 62px" />
        </template>
      </el-table-column>
      <el-table-column label="转弯" width="96">
        <template #default="{ $index }">
          <el-select v-model="items[$index].turn_mode" size="small" style="width: 92px">
            <el-option label="定点" value="fixed" />
            <el-option label="协调" value="coordinated" />
            <el-option label="自适应" value="adaptive" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column label="动作" width="88">
        <template #default="{ $index }">
          <el-select v-model="items[$index].action" size="small" style="width: 84px">
            <el-option label="无" value="none" />
            <el-option label="拍照" value="camera" />
            <el-option label="舵机" value="servo" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="78">
        <template #default="{ $index }">
          <el-button link type="danger" size="small" @click="removeItem($index)">删</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="panel-actions qgc-command-buttons">
      <el-input-number v-model="newAltitude" :min="1" :max="500" size="small" controls-position="right" style="width: 90px" />
      <el-button size="small" @click="addItem">添加航点</el-button>
      <el-button size="small" class="cmd-takeoff" :loading="uploading" :disabled="items.length === 0" @click="onUpload">上传</el-button>
      <el-button size="small" :loading="downloading" @click="onDownload">下载</el-button>
      <el-button size="small" type="danger" plain :loading="clearing" :disabled="items.length === 0" @click="onClear">清除</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { qgcApi } from "@/api";
import type { QgcMissionItem } from "@/qgc/api/qgc";

/**
 * 本地航点类型（在 API 类型基础上扩展 DJI 式航点属性）
 *
 * 上传时仅提交 API 基础字段（seq/command/lat/lon/altitude），
 * 扩展属性（停留时间/转弯模式/动作）用于 UI 规划与 JSON 导入导出。
 */
interface LocalMissionItem extends QgcMissionItem {
  /** 停留时间（秒） */
  hold_time?: number;
  /** 转弯模式：fixed 定点 / coordinated 协调 / adaptive 自适应 */
  turn_mode?: "fixed" | "coordinated" | "adaptive";
  /** 动作：none / camera 拍照 / servo 舵机 */
  action?: "none" | "camera" | "servo";
}

/** 航点列表（上传时服务端自动补首页并重排序号） */
const items = ref<LocalMissionItem[]>([]);
/** 新航点默认高度（米） */
const newAltitude = ref(30);
/** 当前任务状态（idle / uploading / downloading / clearing） */
const progressState = ref("idle");
/** 任务总条目数（含首页） */
const missionTotal = ref(0);
/** 已处理条目数 */
const missionReceived = ref(0);
/** 任务结果描述 */
const missionResult = ref("");

/** 上传操作进行中 */
const uploading = ref(false);
/** 下载操作进行中 */
const downloading = ref(false);
/** 清除操作进行中 */
const clearing = ref(false);

// ---------- 航线模板 / 相对坐标 / 导入导出 ----------

/** 矩形扫描间距（米） */
const templateSpacing = ref(100);
/** 相对坐标编辑器展开 */
const showPolar = ref(false);
/** 相对坐标角度（度） */
const polarAngle = ref(90);
/** 相对坐标距离（米） */
const polarDist = ref(100);
/** 隐藏的文件输入 */
const fileInput = ref<HTMLInputElement | null>(null);

/**
 * 应用航线模板：矩形扫描（在当前末点附近生成 2 行蛇形往返航点）
 *
 * 参考 DJI 6 种预设航线模板中的区域扫描：定高定距、蛇形覆盖。
 */
function applyTemplate() {
  if (items.value.length === 0) {
    ElMessage.warning("请先添加至少一个航点作为起始基准");
    return;
  }
  const last = items.value[items.value.length - 1];
  const spacingDeg = templateSpacing.value / 111320.0;
  // 3 行 × 2 列蛇形
  const plan: [number, number][] = [];
  for (let row = 0; row < 3; row++) {
    const rowLat = last.lat + row * spacingDeg;
    if (row % 2 === 0) {
      plan.push([rowLat, last.lon], [rowLat, last.lon + spacingDeg], [rowLat, last.lon + 2 * spacingDeg]);
    } else {
      plan.push([rowLat, last.lon + 2 * spacingDeg], [rowLat, last.lon + spacingDeg], [rowLat, last.lon]);
    }
  }
  plan.forEach(([lat, lon]) => {
    items.value.push({
      seq: items.value.length + 1,
      command: 16,
      lat,
      lon,
      altitude: newAltitude.value,
      hold_time: 0,
      turn_mode: "adaptive",
      action: "camera",
    });
  });
  renumber();
  ElMessage.success(`已生成 ${plan.length} 个扫描航点（间距 ${templateSpacing.value}m）`);
}

/** 相对坐标：以最后航点为基准，按角度 + 距离生成新航点 */
function addByPolar() {
  if (items.value.length === 0) {
    ElMessage.warning("请先添加基准航点");
    return;
  }
  const last = items.value[items.value.length - 1];
  const rad = (polarAngle.value * Math.PI) / 180;
  const dLat = (polarDist.value * Math.cos(rad)) / 111320.0;
  const dLon = (polarDist.value * Math.sin(rad)) / (111320.0 * Math.max(0.01, Math.abs(Math.cos(last.lat * (Math.PI / 180)))));
  const lat = last.lat + dLat;
  const lon = last.lon + dLon;
  items.value.push({
    seq: items.value.length + 1,
    command: 16,
    lat,
    lon,
    altitude: newAltitude.value,
    hold_time: 0,
    turn_mode: "adaptive",
    action: "none",
  });
  renumber();
}

/** 导出任务为 JSON 文件（含扩展属性） */
function exportMission() {
  if (items.value.length === 0) {
    ElMessage.warning("没有可导出的航点");
    return;
  }
  const payload = items.value.map(({ seq, command, lat, lon, altitude, hold_time, turn_mode, action }) => ({
    seq,
    command,
    lat,
    lon,
    altitude,
    hold_time: hold_time ?? 0,
    turn_mode: turn_mode ?? "adaptive",
    action: action ?? "none",
  }));
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `qgc-mission-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-")}.json`;
  a.click();
  URL.revokeObjectURL(url);
  ElMessage.success("任务已导出");
}

/** 触发导入文件选择 */
function importMission() {
  fileInput.value?.click();
}

/** 导入 JSON 文件 */
function onFileSelected(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    try {
      const parsed = JSON.parse(String(reader.result)) as LocalMissionItem[];
      if (!Array.isArray(parsed) || parsed.length === 0 || parsed.length > 100) {
        throw new Error("文件内容无效");
      }
      items.value = parsed.map((p, i) => ({
        seq: i + 1,
        command: p.command ?? 16,
        lat: p.lat,
        lon: p.lon,
        altitude: p.altitude ?? 30,
        hold_time: p.hold_time ?? 0,
        turn_mode: p.turn_mode ?? "adaptive",
        action: p.action ?? "none",
      }));
      renumber();
      ElMessage.success(`已导入 ${items.value.length} 个航点`);
    } catch {
      ElMessage.error("导入失败：文件格式无效");
    }
  };
  reader.readAsText(file);
}

/** 状态标签颜色 */
const stateTagType = computed(() => {
  switch (progressState.value) {
    case "uploading":
    case "downloading":
    case "clearing":
      return "warning";
    case "idle":
      return missionResult.value === "ok" ? "success" : "info";
    default:
      return "info";
  }
});

/** 状态文字 */
const stateText = computed(() => {
  switch (progressState.value) {
    case "uploading":
      return `上传中 ${missionReceived.value}/${missionTotal.value}`;
    case "downloading":
      return `下载中 ${missionReceived.value}/${missionTotal.value}`;
    case "clearing":
      return "清除中…";
    case "idle":
      if (missionResult.value === "ok") return "任务就绪";
      return "待命";
    default:
      return progressState.value;
  }
});

/** 进度百分比（含首页，total 为 0 时显示 0） */
const progressPct = computed(() => {
  if (missionTotal.value <= 0) return 0;
  return Math.round((missionReceived.value / missionTotal.value) * 100);
});

/** 添加一个航点（位置取最新遥测附近偏移，避免重叠） */
function addItem() {
  const count = items.value.length;
  items.value.push({
    seq: count + 1,
    command: 16,
    lat: 31.2304 + count * 0.0006,
    lon: 121.4737 + count * 0.0006,
    altitude: newAltitude.value,
    hold_time: 0,
    turn_mode: "adaptive",
    action: "none",
  });
  renumber();
}

/** 删除指定航点 */
function removeItem(index: number) {
  items.value.splice(index, 1);
  renumber();
}

/** 按数组顺序重排序号（1 起，首页 seq=0 由服务端自动补） */
function renumber() {
  items.value.forEach((item, i) => {
    item.seq = i + 1;
  });
}

/** 上传任务（仅提交 API 基础字段） */
async function onUpload() {
  uploading.value = true;
  try {
    const payload: QgcMissionItem[] = items.value.map(({ seq, command, lat, lon, altitude }) => ({
      seq,
      command,
      lat,
      lon,
      altitude,
    }));
    const response = await qgcApi.uploadMission(payload);
    if (response.data) {
      ElMessage.success("任务已提交上传");
    } else {
      ElMessage.error("任务上传提交失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "任务上传失败");
  } finally {
    uploading.value = false;
  }
}

/** 从飞控下载当前任务（完成后刷新列表） */
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

/** 清除飞控任务 */
async function onClear() {
  clearing.value = true;
  try {
    const response = await qgcApi.clearMission();
    if (!response.data) {
      ElMessage.error("任务清除提交失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "任务清除失败");
  } finally {
    clearing.value = false;
  }
}

/** 刷新任务快照（进入页面时与下载完成后调用） */
async function refreshMission() {
  try {
    const response = await qgcApi.getMission();
    const data = response.data;
    if (!data) return;
    progressState.value = data.state;
    // 下载完成或空闲时同步航点列表
    if (data.state === "idle" && data.items.length > 0) {
      items.value = data.items.filter((item) => item.seq > 0).map((item) => ({
        ...item,
        hold_time: 0,
        turn_mode: "adaptive" as const,
        action: "none" as const,
      }));
    }
  } catch {
    // 忽略错误（未登录/服务未启动场景）
  }
}

/**
 * 外部注入任务进度事件
 *
 * @param state - 任务状态
 * @param total - 总条目数
 * @param received - 已处理条目数
 * @param result - 结果描述
 */
function applyProgress(state: string, total: number, received: number, result: string) {
  progressState.value = state;
  missionTotal.value = total;
  missionReceived.value = received;
  missionResult.value = result;
  if (state === "idle" && result === "ok") {
    // 上传/下载/清除成功完成后同步快照
    refreshMission();
  }
}

/** 外部清空列表（清除成功后调用） */
function clearAll() {
  items.value = [];
}

/** 外部添加航点（地图点击添加） */
function addExternalItem(lat: number, lon: number) {
  items.value.push({
    seq: items.value.length + 1,
    command: 16,
    lat,
    lon,
    altitude: newAltitude.value,
    hold_time: 0,
    turn_mode: "adaptive",
    action: "none",
  });
  renumber();
}

onMounted(() => {
  refreshMission();
});

defineExpose({
  applyProgress,
  clearAll,
  refreshMission,
  addExternalItem,
  get items() {
    return items.value;
  },
});
</script>

<style scoped>
.mission-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-title {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 2px;
  color: var(--text-accent);
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: linear-gradient(180deg, #00d4ff, #0077b6);
  box-shadow: 0 0 8px rgba(0, 180, 216, 0.8);
}

.mission-progress {
  width: 100%;
}

.mission-progress :deep(.el-progress-bar__outer) {
  background: var(--bg-cell);
}

/* 工具行 */
.tool-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding: 6px 8px;
  background: rgba(7, 13, 26, 0.5);
  border: 1px solid rgba(30, 58, 95, 0.6);
  border-radius: 6px;
}

.tool-hint {
  font-size: 11px;
  color: var(--text-dim);
}

/* 相对坐标编辑器 */
.polar-editor {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding: 6px 8px;
  background: rgba(7, 13, 26, 0.5);
  border: 1px solid rgba(240, 192, 64, 0.4);
  border-radius: 6px;
}

.polar-label {
  font-size: 11px;
  letter-spacing: 1px;
  color: var(--text-dim);
}

/* 航点表格：深色玻璃拟态 */
.wp-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: var(--bg-card-header);
  --el-table-header-text-color: var(--text-accent);
  --el-table-border-color: var(--border-color);
  --el-table-row-hover-bg-color: var(--bg-hover);
  --el-table-text-color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.wp-table :deep(th.el-table__cell) {
  background: var(--bg-card-header) !important;
  color: var(--text-accent) !important;
  font-size: 12px;
  letter-spacing: 1px;
}

.wp-table :deep(td.el-table__cell) {
  background: rgba(10, 20, 40, 0.6) !important;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 12px;
}

.wp-table :deep(.el-table__inner-wrapper::before) {
  display: none;
}

.panel-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
</style>
