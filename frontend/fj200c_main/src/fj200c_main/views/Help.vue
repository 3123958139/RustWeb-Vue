<!--
  Help.vue — 帮助文档页面（fj200c_main 模块）

  读取后端 help_doc.md 并渲染为 HTML（支持 Markdown 语法）。

  适配说明（Tauri → Web）：
  - 原版通过 commands.readReadmeFile() 读取 README.md
  - Web 版改为 fj200cMainApi.getHelp() 读取 help_doc.md（编译期内嵌）
  - 原版依赖 mermaid 库，Web 版省略 mermaid（依赖未引入，仅渲染普通 Markdown）
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page">
        <div class="fm-panel help-panel">
          <div class="fm-panel-header">用户操作说明</div>
          <div class="fm-panel-body" v-loading="loading">
            <div v-if="error" class="error">{{ error }}</div>
            <div v-else class="markdown-body" v-html="renderedHtml"></div>
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import MarkdownIt from 'markdown-it'
import { fj200cMainApi } from '@/api'
import ScaledPage from '@/fj200c_main/components/ScaledPage.vue'

const loading = ref(true)
const error = ref('')
const renderedHtml = ref('')

const md = new MarkdownIt({
  html: true,
  breaks: true,
})

onMounted(async () => {
  try {
    const response = await fj200cMainApi.getHelp()
    if (response.success && response.data) {
      renderedHtml.value = md.render(response.data.content || '')
    } else {
      error.value = response.message || '加载帮助文档失败'
    }
  } catch (err: any) {
    error.value = err.response?.data?.message || '加载帮助文档失败'
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

/* 帮助面板撑满舞台，正文内部滚动 */
.help-panel {
  flex: 1;
  min-height: 0;
  margin-bottom: 0;
  display: flex;
  flex-direction: column;
}

.help-panel .fm-panel-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.error {
  text-align: center;
  padding: 48px;
  font-size: 16px;
  color: var(--fm-danger);
}

.markdown-body {
  color: var(--fm-text);
  padding: 8px 0;
  line-height: 1.7;
}

.markdown-body :deep(h1) {
  font-size: 1.8em;
  margin: 0.67em 0;
  color: var(--fm-text);
  border-bottom: 1px solid var(--fm-border);
  padding-bottom: 0.3em;
}

.markdown-body :deep(h2) {
  font-size: 1.5em;
  margin: 0.75em 0;
  color: var(--fm-text);
  border-bottom: 1px solid var(--fm-border);
  padding-bottom: 0.3em;
}

.markdown-body :deep(h3) {
  font-size: 1.25em;
  margin: 0.83em 0;
  color: var(--fm-text);
}

.markdown-body :deep(p) {
  margin: 0.5em 0;
  line-height: 1.7;
}

.markdown-body :deep(code) {
  background: var(--fm-code-bg);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: Consolas, monospace;
  font-size: 0.9em;
  color: var(--fm-accent);
}

.markdown-body :deep(pre) {
  background: var(--fm-code-bg);
  padding: 16px;
  border-radius: 4px;
  overflow-x: auto;
  border: 1px solid var(--fm-border);
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  color: var(--fm-text);
}

.markdown-body :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid var(--fm-border);
  padding: 8px 12px;
  text-align: left;
}

.markdown-body :deep(th) {
  background: var(--fm-panel-alt);
  font-weight: bold;
  color: var(--fm-text);
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
  border-left: 4px solid var(--fm-accent);
  padding-left: 1em;
  margin: 0.5em 0;
  color: var(--fm-text-secondary);
}

.markdown-body :deep(a) {
  color: var(--fm-accent);
  text-decoration: none;
}

.markdown-body :deep(a:hover) {
  text-decoration: underline;
}
</style>
