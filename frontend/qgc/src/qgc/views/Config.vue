<!--
  配置文件编辑器（qgc Config）

  读取/编辑/保存 config-qgc.ini（保存后需重启服务生效）。
-->
<template>
  <div class="qgc-config-root">
    <div class="qgc-config-toolbar">
      <span class="toolbar-title">配置文件 (config-qgc.ini)</span>
      <div class="spacer"></div>
      <el-button size="small" @click="loadConfig">刷新</el-button>
      <el-button type="primary" size="small" :loading="saving" @click="saveConfig">保存</el-button>
    </div>
    <el-input
      v-model="configContent"
      type="textarea"
      :rows="30"
      spellcheck="false"
      class="config-editor"
      placeholder="加载中…"
    />
    <p class="config-hint">修改保存后需重启服务生效（服务运行时修改立即生效的仅 [Mock] 开关）。</p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { qgcApi } from "@/api";

/** 配置文件内容（可编辑） */
const configContent = ref("");
/** 保存加载状态 */
const saving = ref(false);

/** 读取配置文件内容 */
async function loadConfig() {
  try {
    const response = await qgcApi.getConfig();
    configContent.value = response.data?.content ?? "";
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "读取配置失败");
  }
}

/** 保存配置文件 */
async function saveConfig() {
  saving.value = true;
  try {
    await qgcApi.saveConfig(configContent.value);
    ElMessage.success("配置已保存（重启服务后生效）");
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "保存配置失败");
  } finally {
    saving.value = false;
  }
}

onMounted(loadConfig);
</script>

<style scoped>
.qgc-config-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 16px;
  box-sizing: border-box;
  gap: 12px;
  background: #17181b;
}

.qgc-config-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toolbar-title {
  font-size: 16px;
  font-weight: 600;
  color: #e0e0e0;
}

.spacer { flex: 1; }

.config-editor {
  flex: 1;
  min-height: 300px;
}

.config-editor :deep(textarea) {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 13px;
  background-color: #141518;
  color: #e0e0e0;
}

.config-hint {
  font-size: 12px;
  color: #909399;
  margin: 0;
}
</style>
