<!--
  数据记录页（CSV 文件浏览）

  功能：
  1. 左侧文件列表：从后端获取 CSV 文件列表
  2. 右侧文件内容：选中文件后解析 CSV 并用 el-table 展示
  3. 简易 CSV 解析器：支持引号包裹的含逗号字段
-->
<template>
  <div class="fj200c_information-root">
    <div class="fj200c_information-page">
      <div class="fj200c_information-toolbar">
        <span class="toolbar-title">数据记录</span>
        <el-tag size="small" type="info">{{ csvDir }}</el-tag>
        <div class="spacer"></div>
        <ServiceNavButton />
        <el-button size="small" :loading="loading" @click="loadFiles">刷新列表</el-button>
      </div>

      <div class="fj200c_information-data-layout">
        <!-- 文件列表 -->
        <div class="fj200c_information-panel fj200c_information-file-list">
          <div class="fj200c_information-panel-header">CSV 文件</div>
          <div class="fj200c_information-panel-body">
            <div v-if="files.length === 0" class="file-empty">暂无数据记录</div>
            <div
              v-for="file in files"
              :key="file"
              class="file-item"
              :class="{ active: file === selectedFile }"
              @click="selectFile(file)"
            >
              <el-icon><Document /></el-icon>
              <span class="file-name">{{ file }}</span>
            </div>
          </div>
        </div>

        <!-- 文件内容 -->
        <div class="fj200c_information-panel">
          <div class="fj200c_information-panel-header">
            {{ selectedFile || "选择文件" }}
            <div class="spacer"></div>
            <span v-if="rows.length > 0" class="row-count">共 {{ rows.length }} 行</span>
          </div>
          <div class="fj200c_information-panel-body">
            <div v-if="loadingContent" class="file-empty">加载中…</div>
            <!-- el-table 动态列：v-for 循环渲染列，列名来自 CSV 头部 -->
            <el-table
              v-else-if="rows.length > 0"
              :data="rows"
              size="small"
              border
              max-height="560"
              class="fj200c_information-csv-table"
            >
              <el-table-column
                v-for="col in columns"
                :key="col"
                :prop="col"
                :label="col"
                min-width="110"
              />
            </el-table>
            <div v-else class="file-empty">请选择左侧文件查看内容</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Document } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import ServiceNavButton from "@/fj200c_information/components/ServiceNavButton.vue";
import { fj200c_informationApi } from "@/api";

/** CSV 文件列表 */
const files = ref<string[]>([]);
/** CSV 文件目录路径 */
const csvDir = ref("csv");
/** 当前选中的文件名 */
const selectedFile = ref("");
/** CSV 表头（动态列名） */
const columns = ref<string[]>([]);
/** CSV 数据行（每行为一个键值对对象） */
const rows = ref<Record<string, string>[]>([]);
/** 文件列表加载状态 */
const loading = ref(false);
/** 文件内容加载状态 */
const loadingContent = ref(false);

/** 从后端加载 CSV 文件列表 */
const loadFiles = async () => {
  loading.value = true;
  try {
    const response = await fj200c_informationApi.listCsvFiles();
    if (response.success && response.data) {
      files.value = response.data.files;
      csvDir.value = response.data.dir;
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || "获取文件列表失败");
  } finally {
    loading.value = false;
  }
};

/**
 * 选中文件并加载内容
 * @param name - 文件名
 */
const selectFile = async (name: string) => {
  selectedFile.value = name;
  loadingContent.value = true;
  try {
    const response = await fj200c_informationApi.getCsvFile(name);
    if (response.success && response.data) {
      const parsed = parseCSV(response.data.content);
      columns.value = parsed.columns;
      rows.value = parsed.rows;
    } else {
      ElMessage.error(response.message || "读取文件失败");
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || "读取文件失败");
  } finally {
    loadingContent.value = false;
  }
};

/**
 * 简易 CSV 解析器
 *
 * 支持：
 * - 逗号分隔字段
 * - 双引号包裹的字段（内含逗号时不会错误分割）
 * - 双引号转义（"" 表示一个 "）
 *
 * @param text - CSV 原始文本
 * @returns 包含 columns（表头）和 rows（数据行数组）的对象
 */
const parseCSV = (text: string): { columns: string[]; rows: Record<string, string>[] } => {
  // 按行分割，过滤空行
  const lines = text.split(/\r?\n/).filter((line) => line.trim().length > 0);
  if (lines.length === 0) return { columns: [], rows: [] };

  /**
   * 按逗号分割一行 CSV，支持引号包裹字段
   *
   * 状态机实现：
   * - inQuotes = true 时，引号内的逗号不作为分隔符
   * - 遇到 "" 时转义为单个 "
   */
  const splitLine = (line: string): string[] => {
    const result: string[] = [];
    let current = "";
    let inQuotes = false;
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      if (inQuotes) {
        if (ch === '"') {
          if (line[i + 1] === '"') {
            current += '"'; // 转义引号
            i++;
          } else {
            inQuotes = false;
          }
        } else {
          current += ch;
        }
      } else if (ch === '"') {
        inQuotes = true;
      } else if (ch === ",") {
        result.push(current);
        current = "";
      } else {
        current += ch;
      }
    }
    result.push(current);
    return result;
  };

  const columns = splitLine(lines[0]); // 第一行为表头
  const rows = lines.slice(1).map((line) => {
    const cells = splitLine(line);
    const row: Record<string, string> = {};
    columns.forEach((col, index) => {
      row[col] = cells[index] ?? ""; // 缺失字段用空字符串填充
    });
    return row;
  });

  return { columns, rows };
};

/** 组件挂载时加载文件列表 */
onMounted(loadFiles);
</script>

<style scoped>
@import "@/fj200c_information/fj200c_information.css";

.fj200c_information-data-layout {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 16px;
  align-items: start;
}

@media (max-width: 1000px) {
  .fj200c_information-data-layout {
    grid-template-columns: 1fr;
  }
}

.fj200c_information-file-list {
  max-height: 640px;
  overflow-y: auto;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  color: var(--fj200c_information-text);
}

.file-item:hover {
  background: var(--fj200c_information-accent-soft);
}

.file-item.active {
  background: var(--fj200c_information-accent-soft);
  color: var(--fj200c_information-accent);
}

.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-empty {
  padding: 24px 0;
  text-align: center;
  color: var(--fj200c_information-text-secondary);
}

.row-count {
  color: var(--fj200c_information-text-secondary);
  font-size: 13px;
}

.spacer {
  flex: 1;
}
</style>
