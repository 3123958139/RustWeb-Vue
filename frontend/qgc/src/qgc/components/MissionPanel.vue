<!--
  任务面板（MissionPanel）

  航点规划列表：增删/排序/上传/下载/清除。
  上传自动补首页（seq=0），任务协议进度经 WebSocket mission_progress 事件实时显示。
-->
<template>
  <div class="mission-panel">
    <div class="panel-header">
      <span class="panel-title">任务规划</span>
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

    <el-table :data="items" size="small" max-height="280" empty-text="暂无航点（可点击地图添加或手动添加）">
      <el-table-column prop="seq" label="#" width="48" />
      <el-table-column prop="lat" label="纬度" width="110">
        <template #default="{ row }">{{ row.lat.toFixed(6) }}</template>
      </el-table-column>
      <el-table-column prop="lon" label="经度" width="110">
        <template #default="{ row }">{{ row.lon.toFixed(6) }}</template>
      </el-table-column>
      <el-table-column prop="altitude" label="高度(m)" width="76" />
      <el-table-column label="操作" width="110">
        <template #default="{ $index }">
          <el-button link type="primary" size="small" :disabled="$index === 0" @click="moveItem($index, -1)">上移</el-button>
          <el-button link type="primary" size="small" :disabled="$index === items.length - 1" @click="moveItem($index, 1)">下移</el-button>
          <el-button link type="danger" size="small" @click="removeItem($index)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="panel-actions">
      <el-input-number v-model="newAltitude" :min="1" :max="500" size="small" controls-position="right" style="width: 110px" />
      <el-button size="small" @click="addItem">添加航点</el-button>
      <el-button size="small" :loading="uploading" :disabled="items.length === 0" @click="onUpload">上传</el-button>
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

/** 航点列表（上传时服务端自动补首页并重排序号） */
const items = ref<QgcMissionItem[]>([]);
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
  });
  renumber();
}

/** 删除指定航点 */
function removeItem(index: number) {
  items.value.splice(index, 1);
  renumber();
}

/** 移动航点位置（dir = -1 上移 / 1 下移） */
function moveItem(index: number, dir: number) {
  const target = index + dir;
  if (target < 0 || target >= items.value.length) return;
  const [item] = items.value.splice(index, 1);
  items.value.splice(target, 0, item);
  renumber();
}

/** 按数组顺序重排序号（1 起，首页 seq=0 由服务端自动补） */
function renumber() {
  items.value.forEach((item, i) => {
    item.seq = i + 1;
  });
}

/** 上传任务 */
async function onUpload() {
  uploading.value = true;
  try {
    const response = await qgcApi.uploadMission(items.value);
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
      items.value = data.items.filter((item) => item.seq > 0);
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
  gap: 8px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-title {
  font-size: 15px;
  font-weight: 600;
}

.mission-progress {
  width: 100%;
}

.panel-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
</style>
