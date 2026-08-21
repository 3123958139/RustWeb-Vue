<!--
  配置文件编辑器（qgc Config，大屏风格）

  读取/编辑/保存 config-qgc.ini（保存后需重启服务生效）。
  布局与 Monitor 一致：1920×1080 设计稿 CSS scale 缩放，面板沿用深海军蓝主题。
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
            <span class="toolbar-title">通信参数配置</span>
          </div>
          <div class="spacer"></div>
          <el-button size="small" @click="loadConfig">刷新</el-button>
          <el-button type="primary" size="small" class="qgc-service-btn" :loading="saving" @click="saveConfig">保存</el-button>
        </header>

        <!-- 配置面板 -->
        <div class="qgc-panel config-panel">
          <div class="panel-title">config-qgc.ini</div>
          <el-input
            v-model="configContent"
            type="textarea"
            spellcheck="false"
            class="config-editor"
            placeholder="加载中…"
          />
          <p class="config-hint">修改保存后需重启服务生效（服务端仅校验语法，运行中的服务不热加载）。</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { qgcApi } from "@/api";
import { useWindowScale } from "@/qgc/composables/useWindowScale";

const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale();

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
/* ============ 缩放容器（与 Monitor 一致） ============ */

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

.spacer {
  flex: 1;
}

/* ============ 配置面板 ============ */

.config-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.panel-title {
  font-size: 13px;
  letter-spacing: 2px;
  color: var(--text-accent);
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

.config-editor {
  flex: 1;
  min-height: 0;
}

.config-editor :deep(textarea) {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 14px;
  background-color: var(--bg-cell);
  color: var(--text-primary);
  border-color: var(--border-color);
}

.config-hint {
  font-size: 12px;
  color: var(--text-dim);
  margin: 0;
  letter-spacing: 0.5px;
}

@keyframes brand-breathe {
  0%,
  100% {
    opacity: 1;
    box-shadow: 0 0 10px rgba(0, 180, 216, 0.9);
  }
  50% {
    opacity: 0.5;
    box-shadow: 0 0 4px rgba(0, 180, 216, 0.4);
  }
}
</style>
