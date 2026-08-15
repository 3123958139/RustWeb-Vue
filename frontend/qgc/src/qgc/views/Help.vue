<!--
  帮助文档页面（qgc Help，大屏风格）

  读取后端 help_doc.md 并渲染为 HTML（支持 Markdown 语法）。
  布局与 Monitor 一致：1920×1080 设计稿 CSS scale 缩放，主题深海军蓝。
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
            <span class="toolbar-title">用户操作说明</span>
          </div>
        </header>

        <!-- 帮助面板 -->
        <div class="qgc-panel help-panel">
          <div v-loading="loading" class="help-body">
            <div v-if="error" class="error">{{ error }}</div>
            <div v-else class="markdown-body" v-html="renderedHtml"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from "vue";
import MarkdownIt from "markdown-it";
import { qgcApi } from "@/api";
import { useWindowScale } from "@/qgc/composables/useWindowScale";

const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale();

const loading = ref(true);
const error = ref("");
const renderedHtml = ref("");

const md = new MarkdownIt({
  html: true,
  breaks: true,
});

onMounted(async () => {
  try {
    const response = await qgcApi.getHelp();
    if (response.success && response.data) {
      renderedHtml.value = md.render(response.data.content || "");
    } else {
      error.value = response.message || "加载帮助文档失败";
    }
  } catch (err: any) {
    error.value = err.response?.data?.message || "加载帮助文档失败";
  } finally {
    loading.value = false;
  }
});
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

/* ============ 帮助面板 ============ */

.help-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.help-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 6px;
  color: var(--text-primary);
}

.error {
  text-align: center;
  padding: 48px;
  font-size: 16px;
  color: var(--text-danger);
}

.markdown-body {
  line-height: 1.7;
  padding: 8px 0;
}

.markdown-body :deep(h1) {
  font-size: 1.8em;
  margin: 0.67em 0;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 0.3em;
}

.markdown-body :deep(h2) {
  font-size: 1.5em;
  margin: 0.75em 0;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 0.3em;
}

.markdown-body :deep(h3) {
  font-size: 1.25em;
  margin: 0.83em 0;
  color: var(--text-primary);
}

.markdown-body :deep(p) {
  margin: 0.5em 0;
  line-height: 1.7;
}

.markdown-body :deep(code) {
  background: var(--bg-hover);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: Consolas, monospace;
  font-size: 0.9em;
  color: var(--text-accent);
}

.markdown-body :deep(pre) {
  background: var(--bg-cell);
  padding: 16px;
  border-radius: 4px;
  overflow-x: auto;
  border: 1px solid var(--border-color);
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  color: var(--text-primary);
}

.markdown-body :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid var(--border-color);
  padding: 8px 12px;
  text-align: left;
}

.markdown-body :deep(th) {
  background: var(--bg-card-header);
  font-weight: bold;
  color: var(--text-primary);
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 2em;
  margin: 0.5em 0;
}

.markdown-body :deep(li) {
  margin: 0.25em 0;
}

.markdown-body :deep(blockquote) {
  border-left: 4px solid var(--text-accent);
  padding-left: 1em;
  margin: 0.5em 0;
  color: var(--text-dim);
}

.markdown-body :deep(a) {
  color: var(--text-accent);
  text-decoration: none;
}

.markdown-body :deep(a:hover) {
  text-decoration: underline;
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
