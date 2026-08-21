<!--
  参数表面板（qgc ParamPanel）

  读取并编辑飞控参数（ArduCopter 精简子集）。对应后端
  GET/PUT /api/qgc/param，模拟器维护全局参数表，写入即时生效。
  布局参考 Mission Planner「配置/可调参数」：参数名 + 值编辑 + 保存。
-->
<template>
  <div class="param-panel">
    <div class="pp-toolbar">
      <el-button size="small" :loading="loading" @click="load">刷新</el-button>
      <span class="pp-hint">修改后点「保存」即时生效（模拟器共享参数表）</span>
    </div>
    <el-table :data="params" size="small" height="420" v-loading="loading" empty-text="暂无参数">
      <el-table-column prop="id" label="参数名" width="220" />
      <el-table-column label="值" width="180">
        <template #default="{ row }">
          <el-input-number
            v-model="draft[row.id]"
            :step="0.01"
            :controls="false"
            size="small"
            controls-position="right"
          />
        </template>
      </el-table-column>
      <el-table-column label="操作" width="100">
        <template #default="{ row }">
          <el-button
            size="small"
            type="primary"
            :loading="savingId === row.id"
            :disabled="draft[row.id] === row.value"
            @click="save(row)"
          >
            保存
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";
import type { QgcParam } from "@/qgc/api/qgc";
import { ElMessage } from "element-plus";

const qgcApi = createQgcApi();

const params = ref<QgcParam[]>([]);
const loading = ref(false);
const savingId = ref<string | null>(null);
const draft = ref<Record<string, number>>({});

async function load() {
  loading.value = true;
  try {
    const res = await qgcApi.getParams();
    if (res.data?.params) {
      params.value = res.data.params;
      draft.value = {};
      for (const p of params.value) draft.value[p.id] = p.value;
    }
  } finally {
    loading.value = false;
  }
}

async function save(p: QgcParam) {
  savingId.value = p.id;
  try {
    const v = draft.value[p.id];
    const res = await qgcApi.setParam(p.id, v);
    if (res.success) {
      p.value = v;
      ElMessage.success(`参数 ${p.id} 已更新为 ${v}`);
    } else {
      ElMessage.error(res.message || "保存失败");
    }
  } finally {
    savingId.value = null;
  }
}

onMounted(load);
</script>

<style scoped>
.param-panel {
  display: flex;
  flex-direction: column;
}
.pp-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}
.pp-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
