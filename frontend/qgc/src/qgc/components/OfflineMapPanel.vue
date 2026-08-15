<!--
  离线地图面板（OfflineMapPanel）

  地图瓦片离线保存与加载管理：
  - 后端代理接口（GET /api/qgc/tiles/{z}/{x}/{y}）命中磁盘缓存直接返回（离线加载），
    未命中则从瓦片源下载并落盘（离线保存）；地图浏览即自动缓存。
  - 本面板提供区域批量保存（中心点 + 半径 + 缩放级别范围，前端计算瓦片坐标后
    并发请求触发后端逐张落盘）、缓存统计与清除。

  说明：Web Mercator 瓦片坐标换算（标准公式）——
  x = floor((lon + 180) / 360 × 2^z)
  y = floor((1 - ln(tan(lat) + sec(lat)) / π) / 2 × 2^z)
-->
<template>
  <div class="offline-panel">
    <div class="panel-header">
      <span class="panel-title">
        <i class="title-dot"></i>
        离线地图
      </span>
      <el-tag :type="stats.count > 0 ? 'success' : 'info'" size="small" class="qgc-state-tag">
        {{ stats.count > 0 ? `已缓存 ${stats.count.toLocaleString()} 张` : "暂无缓存" }}
      </el-tag>
    </div>

    <!-- 缓存统计 -->
    <div class="stat-row">
      <span class="stat-item">瓦片 <b>{{ stats.count.toLocaleString() }}</b></span>
      <span class="stat-item">占用 <b>{{ formatBytes(stats.bytes) }}</b></span>
      <el-button size="small" type="danger" plain :loading="clearing" :disabled="stats.count === 0 || saving" @click="onClear">
        清除缓存
      </el-button>
    </div>

    <!-- 保存参数 -->
    <div class="save-form">
      <div class="form-row">
        <span class="form-label">中心纬度</span>
        <el-input-number v-model="centerLat" :min="-90" :max="90" :precision="6" :step="0.001" size="small" controls-position="right" style="width: 130px" />
        <span class="form-label">经度</span>
        <el-input-number v-model="centerLon" :min="-180" :max="180" :precision="6" :step="0.001" size="small" controls-position="right" style="width: 130px" />
      </div>
      <div class="form-row">
        <span class="form-label">半径(km)</span>
        <el-radio-group v-model="radiusKm" size="small">
          <el-radio-button :value="0.5">0.5</el-radio-button>
          <el-radio-button :value="1">1</el-radio-button>
          <el-radio-button :value="2">2</el-radio-button>
          <el-radio-button :value="5">5</el-radio-button>
        </el-radio-group>
        <span class="form-label">缩放</span>
        <el-input-number v-model="minZoom" :min="5" :max="18" size="small" controls-position="right" style="width: 84px" />
        <span class="form-label">至</span>
        <el-input-number v-model="maxZoom" :min="6" :max="19" size="small" controls-position="right" style="width: 84px" />
      </div>
      <div class="estimate-row">
        <span class="est-text">
          预计 <b>{{ estimateTiles().toLocaleString() }}</b> 张 ≈ <b>{{ formatBytes(estimateTiles() * 25 * 1024) }}</b>
          <el-tooltip content="按平均 25KB/张估算，实际以瓦片内容为准" placement="top">
            <i class="hint-icon">?</i>
          </el-tooltip>
        </span>
        <el-button size="small" type="primary" class="qgc-save-btn" :loading="saving" :disabled="estimateTiles() === 0 || estimateTiles() > 30000" @click="onSave">
          保存离线地图
        </el-button>
      </div>
      <div v-if="estimateTiles() > 30000" class="warn-text">瓦片数量超过 3 万张，请缩小半径或降低最大缩放级别</div>
      <div v-if="saving" class="progress-wrap">
        <el-progress :percentage="progressPct" :stroke-width="12" :status="failed > 0 && failed === done ? 'exception' : 'success'" class="save-progress" />
        <span class="progress-text">{{ done.toLocaleString() }} / {{ total.toLocaleString() }}（失败 {{ failed }}）</span>
        <el-button size="small" @click="cancelSaving">取消</el-button>
      </div>
      <div v-if="!saving && done > 0" class="finish-text">保存完成：成功 {{ done - failed }} 张，失败 {{ failed }} 张（离线加载自动命中缓存）</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { qgcApi } from "@/api";

/** 初始中心点（由父组件传入地图中心或遥测位置；缺省上海） */
const props = defineProps<{ center?: [number, number] | null }>();

/** 瓦片缓存统计（数量 + 磁盘字节） */
const stats = ref({ count: 0, bytes: 0 });
/** 保存中心点（度） */
const centerLat = ref(31.2304);
const centerLon = ref(121.4737);
/** 保存半径（公里） */
const radiusKm = ref(0.5);
/** 缩放级别范围 */
const minZoom = ref(12);
const maxZoom = ref(15);
/** 保存进度 */
const saving = ref(false);
const done = ref(0);
const failed = ref(0);
const total = ref(0);
let cancelFlag = false;
const clearing = ref(false);

const progressPct = computed(() => (total.value > 0 ? Math.round((done.value / total.value) * 100) : 0));

/** 经纬度 → Web Mercator 瓦片坐标 */
function latLonToTile(lat: number, lon: number, z: number): [number, number] {
  const n = 2 ** z;
  const x = Math.floor(((lon + 180) / 360) * n);
  const latRad = (lat * Math.PI) / 180;
  const y = Math.floor(((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2) * n);
  return [x, y];
}

/** 计算区域内全部瓦片（含范围校验与 x 环绕、y 夹紧） */
function collectTiles(): Array<[number, number, number]> {
  const tiles: Array<[number, number, number]> = [];
  if (minZoom.value > maxZoom.value) return tiles;
  const dLat = radiusKm.value / 110.574;
  const dLon = radiusKm.value / (111.32 * Math.cos((centerLat.value * Math.PI) / 180) || 111.32);
  for (let z = minZoom.value; z <= maxZoom.value; z++) {
    const n = 2 ** z;
    const [xMin, yMin] = latLonToTile(centerLat.value + dLat, centerLon.value - dLon, z);
    const [xMax, yMax] = latLonToTile(centerLat.value - dLat, centerLon.value + dLon, z);
    for (let x = xMin; x <= xMax; x++) {
      const xw = ((x % n) + n) % n;
      for (let y = Math.max(0, yMin); y <= Math.min(n - 1, yMax); y++) {
        tiles.push([z, xw, y]);
      }
    }
  }
  return tiles;
}

/** 预估瓦片数量（表单驱动，无副作用） */
function estimateTiles(): number {
  return collectTiles().length;
}

/** 字节数格式化 */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

/** 拉取缓存统计 */
async function loadStats() {
  try {
    const resp = await qgcApi.getTileStats();
    if (resp.data) stats.value = resp.data;
  } catch {
    // 统计失败不阻塞面板
  }
}

/** 保存离线地图：并发请求瓦片端点触发后端逐张下载落盘 */
async function onSave() {
  const tiles = collectTiles();
  if (tiles.length === 0) {
    ElMessage.warning("参数范围内的瓦片数量为 0，请调整半径或缩放级别");
    return;
  }
  if (tiles.length > 30000) {
    ElMessage.warning("瓦片数量超过 3 万张，请缩小范围");
    return;
  }
  saving.value = true;
  done.value = 0;
  failed.value = 0;
  total.value = tiles.length;
  cancelFlag = false;
  const concurrency = 8;
  let next = 0;
  const workers = Array.from({ length: concurrency }, async () => {
    while (next < tiles.length && !cancelFlag) {
      const [z, x, y] = tiles[next++];
      try {
        const resp = await fetch(qgcApi.buildTileUrl(z, x, y));
        if (!resp.ok) failed.value++;
      } catch {
        failed.value++;
      }
      done.value++;
    }
  });
  await Promise.all(workers);
  saving.value = false;
  if (cancelFlag) {
    ElMessage.info(`已取消，本次保存 ${done.value - failed.value} 张`);
  } else {
    ElMessage.success(`离线地图保存完成（成功 ${done.value - failed.value} 张，失败 ${failed.value} 张）`);
  }
  await loadStats();
}

/** 取消保存（等待当前批次请求返回后停止） */
function cancelSaving() {
  cancelFlag = true;
}

/** 清除缓存（二次确认） */
async function onClear() {
  try {
    await ElMessageBox.confirm("将删除磁盘上全部已缓存瓦片，离线地图将不可用。确定继续？", "清除瓦片缓存", {
      confirmButtonText: "清除",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    return;
  }
  clearing.value = true;
  try {
    const resp = await qgcApi.clearTiles();
    if (resp.data?.saved) {
      ElMessage.success("瓦片缓存已清除");
      stats.value = { count: 0, bytes: 0 };
      done.value = 0;
      failed.value = 0;
    } else {
      ElMessage.error(resp.message || "清除失败");
    }
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "清除失败");
  } finally {
    clearing.value = false;
  }
}

onMounted(() => {
  if (props.center) {
    centerLat.value = props.center[0];
    centerLon.value = props.center[1];
  }
  loadStats();
});
</script>

<style scoped>
.offline-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 1px;
  color: var(--text-primary);
}

.title-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-accent);
  box-shadow: 0 0 8px rgba(0, 180, 216, 0.8);
}

.stat-row {
  display: flex;
  align-items: center;
  gap: 14px;
  font-size: 12px;
  color: var(--text-dim);
}

.stat-item b {
  color: var(--text-primary);
  font-family: "Consolas", "Courier New", monospace;
}

.save-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.form-label {
  font-size: 12px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.estimate-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 2px;
}

.est-text {
  font-size: 12px;
  color: var(--text-dim);
}

.est-text b {
  color: var(--text-accent);
  font-family: "Consolas", "Courier New", monospace;
}

.hint-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  margin-left: 4px;
  border-radius: 50%;
  border: 1px solid var(--border-color);
  font-style: normal;
  font-size: 10px;
  color: var(--text-dim);
  cursor: help;
}

.warn-text {
  font-size: 12px;
  color: #f0a040;
}

.progress-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
}

.save-progress {
  flex: 1;
  min-width: 0;
}

.progress-text {
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  font-family: "Consolas", "Courier New", monospace;
}

.finish-text {
  font-size: 12px;
  color: var(--text-success);
}
</style>

<style>
/* Element Plus 组件深色主题适配（非 scoped，覆盖 dialog 内组件） */
.qgc-state-tag {
  border-radius: 4px;
}
</style>