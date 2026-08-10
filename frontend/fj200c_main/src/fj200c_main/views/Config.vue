<!--
  Config.vue — 配置文件编辑页面（fj200c_main 模块）

  挂载时通过 useConfigDialog 读取 config-fj200c_main.ini，
  以全屏页面展示文本编辑器，点击保存写回文件。

  适配说明（Tauri → Web）：
  - 原版独立子窗口 800×600，Web 版改为应用内路由页面
  - 原版通过 commands.readConfigFile/saveConfigFile，Web 版改为 fj200cMainApi.getConfig/saveConfig
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page">
        <div class="fm-panel config-panel">
          <div class="fm-panel-header">
            <span>config-fj200c_main.ini</span>
            <div class="spacer"></div>
            <el-button @click="handleSave" type="primary" :loading="saving">保存</el-button>
          </div>
          <div class="fm-panel-body editor-body" v-loading="loading">
            <el-input
              v-model="content"
              type="textarea"
              :rows="28"
              class="config-editor"
              placeholder="# 配置文件内容"
            />
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script lang="ts" setup>
import { onMounted } from "vue";
import { ElMessage } from "element-plus";
import { useConfigDialog } from "@/fj200c_main/composables/useConfigDialog";
import ScaledPage from "@/fj200c_main/components/ScaledPage.vue";

const { content, loading, saving, open, save } = useConfigDialog();

onMounted(() => {
  open();
});

async function handleSave() {
  const result = await save();
  if (result.success) {
    ElMessage.success("保存成功");
  } else {
    ElMessage.error(result.message || "保存失败");
  }
}
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

.config-panel {
  flex: 1;
  min-height: 0;
  margin-bottom: 0;
  display: flex;
  flex-direction: column;
}

.fm-panel-header {
  justify-content: space-between;
}

.fm-panel-header .spacer {
  flex: 1;
}

.editor-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.editor-body :deep(.el-textarea) {
  height: 100%;
  flex: 1;
}

.editor-body :deep(.el-textarea__wrapper) {
  height: 100% !important;
  background: var(--fm-code-bg);
  border: 1px solid var(--fm-border);
}

.editor-body :deep(.el-textarea__inner) {
  height: 100% !important;
  resize: none;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 13px;
  color: var(--fm-text);
  background: var(--fm-code-bg);
}
</style>
