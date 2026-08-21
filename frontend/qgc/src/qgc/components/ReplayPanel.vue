<!--
  遥测 CSV 回放面板（qgc ReplayPanel）

  列出后端按天记录的遥测 CSV 文件（GET /api/qgc/telemetry/csv），
  选定后下载并解析，按行推进回放，显示关键遥测随时间变化。
  纯前端解析，无需飞控/服务参与。
-->
<template>
  <div class="replay-panel">
    <div class="rp-row">
      <el-select v-model="selected" placeholder="选择 CSV 文件" size="small" class="rp-select" @change="onSelect">
        <el-option v-for="f in files" :key="f.name" :label="f.name" :value="f.name" />
      </el-select>
      <el-button size="small" :disabled="!rows.length" @click="togglePlay">{{ playing ? "暂停" : "播放" }}</el-button>
      <el-button size="small" :disabled="!rows.length" @click="stop">停止</el-button>
    </div>
    <div class="rp-progress" v-if="rows.length">
      <input type="range" min="0" :max="rows.length - 1" v-model.number="cursor" class="rp-slider" @input="onSeek" />
      <span class="rp-idx">{{ cursor + 1 }} / {{ rows.length }}</span>
    </div>
    <div class="rp-grid" v-if="current">
      <div><span>时间</span><b>{{ current.t }}</b></div>
      <div><span>高度</span><b>{{ current.alt?.toFixed(1) }} m</b></div>
      <div><span>电压</span><b>{{ current.volt?.toFixed(2) }} V</b></div>
      <div><span>速度</span><b>{{ current.spd?.toFixed(1) }} m/s</b></div>
      <div><span>模式</span><b>{{ current.mode }}</b></div>
      <div><span>GNSS</span><b>{{ current.fix }} ({{ current.sat }})</b></div>
    </div>
    <div class="rp-empty" v-else>暂无数据，请先运行服务并录制遥测（G2）</div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onUnmounted } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";
import type { QgcCsvFile } from "@/qgc/api/qgc";

const qgcApi = createQgcApi();

const files = ref<QgcCsvFile[]>([]);
const selected = ref("");
const rows = ref<any[]>([]);
const cursor = ref(0);
const playing = ref(false);
let timer: number | undefined;

interface Row {
  t: string;
  alt?: number;
  volt?: number;
  spd?: number;
  mode: string;
  fix: number;
  sat: number;
}

const current = reactive<Row>({ t: "", mode: "", fix: 0, sat: 0 });

function parse(text: string) {
  const lines = text.split(/\r?\n/).filter((l) => l.trim().length);
  if (lines.length < 2) return [];
  const header = lines[0].split(",");
  const idx = (name: string) => header.indexOf(name);
  const iAlt = idx("alt_m"); const iVolt = idx("voltage_v"); const iSpd = idx("speed_m_s");
  const iMode = idx("mode"); const iFix = idx("gnss_fix"); const iSat = idx("satellites");
  const iTime = idx("timestamp") >= 0 ? idx("timestamp") : 0;
  return lines.slice(1).map((l) => {
    const c = l.split(",");
    const num = (i: number) => (i >= 0 && c[i] !== undefined && c[i] !== "" ? Number(c[i]) : undefined);
    return {
      t: iTime >= 0 ? c[iTime] ?? "" : "",
      alt: num(iAlt),
      volt: num(iVolt),
      spd: num(iSpd),
      mode: iMode >= 0 ? c[iMode] ?? "" : "",
      fix: num(iFix) ?? 0,
      sat: num(iSat) ?? 0,
    } as Row;
  });
}

async function refresh() {
  try {
    const r = await qgcApi.listCsv();
    files.value = (r.data as unknown as QgcCsvFile[]) ?? [];
  } catch {
    files.value = [];
  }
}

async function onSelect(name: string) {
  stop();
  try {
    const text = await qgcApi.getCsv(name);
    rows.value = parse(typeof text === "string" ? text : String(text));
    cursor.value = 0;
    applyCursor();
  } catch {
    rows.value = [];
  }
}

function applyCursor() {
  const r = rows.value[cursor.value];
  if (r) Object.assign(current, r);
}

function onSeek() {
  applyCursor();
}

function tick() {
  if (cursor.value < rows.value.length - 1) {
    cursor.value++;
    applyCursor();
  } else {
    stop();
  }
}

function togglePlay() {
  if (playing.value) {
    stop();
  } else if (rows.value.length) {
    playing.value = true;
    timer = window.setInterval(tick, 80);
  }
}

function stop() {
  playing.value = false;
  if (timer !== undefined) {
    clearInterval(timer);
    timer = undefined;
  }
}

onUnmounted(stop);
refresh();
</script>

<style scoped>
.replay-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.rp-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.rp-select {
  flex: 1;
}
.rp-progress {
  display: flex;
  align-items: center;
  gap: 8px;
}
.rp-slider {
  flex: 1;
}
.rp-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
  font-size: 13px;
}
.rp-grid span {
  color: var(--el-text-color-secondary);
  margin-right: 6px;
}
.rp-grid b {
  font-variant-numeric: tabular-nums;
}
.rp-empty {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
