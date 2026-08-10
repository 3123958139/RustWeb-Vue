<!--
  Data.vue — 数据浏览页面（fj200c_main 模块）

  支持从后端 CSV 目录选择文件并解析展示为表格，自动处理引号转义和换行。
  沿用现有 fj200c_information 模块的 Web 模式：通过 API 获取 CSV 文件列表与内容。

  适配说明（Tauri → Web）：
  - 原版通过 <input type="file"> 读取本地 CSV 文件
  - Web 版改为从后端 csv 目录选择文件（fj200cMainApi.listCsvFiles / getCsvFile）
  - 原版通过 commands.saveTextFile() 另存为 TXT，Web 版改为前端 Blob 下载
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page">
        <div class="fm-data-layout">
          <!-- 左侧：CSV 文件列表 -->
          <div class="fm-panel data-panel">
            <div class="fm-panel-header">
              <span>CSV 文件列表</span>
              <el-button size="small" @click="loadFileList" :loading="fileLoading">刷新</el-button>
            </div>
            <div class="fm-panel-body fm-file-list" v-loading="fileLoading">
              <div
                v-for="f in csvFiles"
                :key="f"
                class="file-item"
                :class="{ active: selectedFile === f }"
                @click="onSelectFile(f)"
              >
                <el-icon><Document /></el-icon>
                <span class="file-name">{{ f }}</span>
              </div>
              <div v-if="!fileLoading && csvFiles.length === 0" class="file-empty">
                暂无 CSV 文件
              </div>
            </div>
          </div>

          <!-- 右侧：CSV 内容表格 -->
          <div class="fm-panel data-panel">
            <div class="fm-panel-header">
              <span>数据浏览</span>
              <div class="spacer"></div>
              <span v-if="selectedFile" class="file-name">{{ selectedFile }}</span>
              <span v-if="rows.length" class="row-count">共 {{ rows.length }} 行</span>
              <el-button
                v-if="rows.length"
                size="small"
                type="primary"
                @click="saveAsTxt"
              >
                另存为 TXT
              </el-button>
            </div>
            <div class="fm-panel-body table-body" v-loading="contentLoading">
              <el-table
                v-if="columns.length"
                :data="rows"
                border
                stripe
                height="100%"
                style="width: 100%"
              >
                <el-table-column
                  v-for="col in columns"
                  :key="col"
                  :prop="col"
                  :label="col"
                  min-width="120"
                  show-overflow-tooltip
                />
              </el-table>
              <div v-else-if="!contentLoading" class="empty-hint">
                请从左侧选择 CSV 文件以浏览数据
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { Document } from "@element-plus/icons-vue";
import { fj200cMainApi } from "@/api";
import ScaledPage from "@/fj200c_main/components/ScaledPage.vue";

const csvFiles = ref<string[]>([]);
const selectedFile = ref("");
const columns = ref<string[]>([]);
const rows = ref<Record<string, string>[]>([]);
const fileLoading = ref(false);
const contentLoading = ref(false);

/** 加载 CSV 文件列表 */
async function loadFileList() {
  fileLoading.value = true;
  try {
    const response = await fj200cMainApi.listCsvFiles();
    if (response.success && response.data) {
      csvFiles.value = response.data.files;
    } else {
      ElMessage.error(response.message || "加载文件列表失败");
    }
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || "加载文件列表失败");
  } finally {
    fileLoading.value = false;
  }
}

/** 选中 CSV 文件并加载内容 */
async function onSelectFile(name: string) {
  selectedFile.value = name;
  columns.value = [];
  rows.value = [];
  contentLoading.value = true;
  try {
    const response = await fj200cMainApi.getCsvFile(name);
    if (response.success && response.data) {
      const result = parseCSV(response.data.content);
      columns.value = result.columns;
      rows.value = result.rows;
    } else {
      ElMessage.error(response.message || "读取文件失败");
    }
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || "读取文件失败");
  } finally {
    contentLoading.value = false;
  }
}

/** 另存为 TXT（前端 Blob 下载） */
function saveAsTxt() {
  if (!columns.value.length || !rows.value.length) return;
  try {
    const sep = "\t";
    const lines: string[] = [];
    lines.push(columns.value.join(sep));
    for (const row of rows.value) {
      lines.push(columns.value.map((c) => row[c] ?? "").join(sep));
    }
    const blob = new Blob([lines.join("\r\n")], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = (selectedFile.value.replace(/\.csv$/i, "") || "data") + ".txt";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    ElMessage.success("文件已下载");
  } catch (err: any) {
    ElMessage.error("保存失败");
  }
}

/** 手写 CSV 解析器（支持双引号转义、跨行字段、\r\n 与 \n 换行） */
function parseCSV(text: string): { columns: string[]; rows: Record<string, string>[] } {
  const lines: string[] = [];
  let current = "";
  let inQuote = false;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inQuote) {
      if (ch === '"') {
        if (i + 1 < text.length && text[i + 1] === '"') {
          current += '"';
          i++;
        } else {
          inQuote = false;
        }
      } else {
        current += ch;
      }
    } else {
      if (ch === '"') {
        inQuote = true;
      } else if (ch === "\n") {
        lines.push(current);
        current = "";
      } else if (ch === "\r") {
        // 跳过 \r
      } else {
        current += ch;
      }
    }
  }
  if (current) lines.push(current);

  if (lines.length < 2) return { columns: [], rows: [] };

  const headers = parseLine(lines[0]);
  const cols = headers.map((h) => h.trim());

  const result: Record<string, string>[] = [];
  for (let i = 1; i < lines.length; i++) {
    const values = parseLine(lines[i]);
    if (values.length === 0 || (values.length === 1 && values[0] === "")) continue;
    const row: Record<string, string> = {};
    cols.forEach((col, idx) => {
      row[col] = values[idx] || "";
    });
    result.push(row);
  }

  return { columns: cols, rows: result };
}

/** 解析单行 CSV（支持双引号转义与逗号分隔） */
function parseLine(line: string): string[] {
  const result: string[] = [];
  let current = "";
  let inQuote = false;

  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (inQuote) {
      if (ch === '"') {
        if (i + 1 < line.length && line[i + 1] === '"') {
          current += '"';
          i++;
        } else {
          inQuote = false;
        }
      } else {
        current += ch;
      }
    } else {
      if (ch === '"') {
        inQuote = true;
      } else if (ch === ",") {
        result.push(current);
        current = "";
      } else {
        current += ch;
      }
    }
  }
  result.push(current);
  return result;
}

onMounted(() => {
  loadFileList();
});
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

.fm-panel-header {
  justify-content: space-between;
}

.fm-panel-header .spacer {
  flex: 1;
}

/* 两栏面板均撑满舞台高度，内部各自滚动 */
.fm-data-layout .data-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin-bottom: 0;
}

.fm-data-layout .data-panel .fm-file-list,
.fm-data-layout .data-panel .table-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.fm-data-layout .data-panel .fm-file-list {
  max-height: none;
  padding: 8px;
}

.table-body {
  padding: 0;
  overflow: hidden;
}

.table-body :deep(.el-table) {
  font-size: 12px;
}

.empty-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 0;
  color: var(--fm-text-secondary);
  font-size: 16px;
}
</style>
