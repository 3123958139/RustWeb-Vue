<!--
  数据流配置面板（qgc StreamConfigPanel）

  设置遥测广播频率（Hz），模拟器下一拍生效。对应后端
  GET/POST /api/qgc/stream，全局遥测频率运行时可调。
-->
<template>
  <div class="stream-panel">
    <div class="sp-row">
      <span class="sp-label">当前频率</span>
      <span class="sp-val">{{ currentHz }} Hz</span>
    </div>
    <div class="sp-row">
      <span class="sp-label">遥测频率</span>
      <el-slider v-model="hz" :min="1" :max="50" :step="1" show-input size="small" />
    </div>
    <div class="sp-actions">
      <el-button size="small" @click="load">刷新</el-button>
      <el-button size="small" type="primary" :loading="saving" @click="apply">应用</el-button>
    </div>
    <div class="sp-hint">调整后模拟器下一拍生效（限幅 1~50Hz）</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";
import { ElMessage } from "element-plus";

const qgcApi = createQgcApi();

const hz = ref(10);
const currentHz = ref(10);
const saving = ref(false);

async function load() {
  try {
    const res = await qgcApi.getStream();
    if (res.data) {
      currentHz.value = res.data.hz;
      hz.value = res.data.hz;
    }
  } catch {
    // 忽略加载失败
  }
}

async function apply() {
  saving.value = true;
  try {
    const res = await qgcApi.setStream(hz.value);
    if (res.success && res.data) {
      currentHz.value = res.data.hz;
      ElMessage.success(`遥测频率已设为 ${res.data.hz} Hz`);
    } else {
      ElMessage.error(res.message || "设置失败");
    }
  } finally {
    saving.value = false;
  }
}

onMounted(load);
</script>

<style scoped>
.stream-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.sp-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.sp-label {
  width: 72px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.sp-val {
  font-weight: 600;
}
.sp-row :deep(.el-slider) {
  flex: 1;
}
.sp-actions {
  display: flex;
  gap: 10px;
}
.sp-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
