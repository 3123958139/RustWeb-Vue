<!--
  CsvEditor.vue —— 默认参数表编辑器（protocol_generator 模块）

  参数表（parameters.csv）行编辑：加载服务端默认参数表、上传 CSV 解析导入、
  下载 CSV（UTF-8 BOM）导出、保存到服务端。
-->
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { CsvParameter } from '../types/protocol'
import { CSharpTypes } from '../types/protocol'
import { protocolGeneratorApi } from '@/api'

/** 参数表行 */
const rows = ref<CsvParameter[]>([])
/** 表格选中的行 */
const selectedRows = ref<CsvParameter[]>([])
/** 隐藏的 CSV 文件输入 */
const fileInput = ref<HTMLInputElement>()

/** 生成空行 */
function emptyRow(): CsvParameter {
  return { name: '', alias: '', unit: '', dataType: '', remark: '' }
}

/** 从服务端加载默认参数表 */
async function loadDefault() {
  try {
    const res = await protocolGeneratorApi.getDefaultCsv()
    rows.value = res.data ?? []
  } catch (e: any) {
    ElMessage.error('加载默认 CSV 失败: ' + (e?.response?.data?.message || e))
  }
}

onMounted(async () => {
  await loadDefault()
})

/** 选择本地 CSV 文件 → 上传内容解析导入 */
function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = async () => {
    try {
      const content = String(reader.result ?? '')
      const res = await protocolGeneratorApi.parseCsv(content)
      rows.value = res.data ?? []
      ElMessage.success('CSV 导入成功')
    } catch (e: any) {
      ElMessage.error('CSV 导入失败: ' + (e?.response?.data?.message || e))
    } finally {
      input.value = ''
    }
  }
  reader.onerror = () => {
    ElMessage.error('CSV 导入失败: 文件读取错误')
    input.value = ''
  }
  reader.readAsText(file)
}

function importCsv() {
  fileInput.value?.click()
}

async function save() {
  try {
    await protocolGeneratorApi.saveDefaultCsv(rows.value)
    ElMessage.success('保存成功')
  } catch (e: any) {
    ElMessage.error('保存失败: ' + (e?.response?.data?.message || e))
  }
}

async function exportCsv() {
  try {
    const res = await protocolGeneratorApi.serializeCsv(rows.value)
    const blob = new Blob([res.data?.content ?? ''], { type: 'text/csv;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'parameters.csv'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    ElMessage.success('CSV 导出成功')
  } catch (e: any) {
    ElMessage.error('CSV 导出失败: ' + (e?.response?.data?.message || e))
  }
}

function addRow() {
  rows.value.push(emptyRow())
}

function deleteSelected() {
  if (selectedRows.value.length === 0) {
    ElMessage.warning('请先选择要删除的行')
    return
  }
  const selected = new Set(selectedRows.value)
  rows.value = rows.value.filter(r => !selected.has(r))
}

function clearAll() {
  rows.value = []
}
</script>

<template>
  <div class="csv-editor">
    <div class="header">
      <span class="title">CSV 参数表编辑</span>
      <span class="tip">默认参数表保存于服务器（parameters.csv），供协议编辑的参数名称下拉使用</span>
    </div>

    <input
      ref="fileInput"
      type="file"
      accept=".csv,text/csv"
      style="display: none;"
      @change="onFileSelected"
    />

    <div class="toolbar">
      <el-button type="primary" @click="save">保存</el-button>
      <el-button @click="importCsv">导入 CSV</el-button>
      <el-button @click="exportCsv">导出 CSV</el-button>
      <el-divider direction="vertical" />
      <el-button @click="addRow">+ 新增行</el-button>
      <el-button type="danger" @click="deleteSelected">× 删除选中</el-button>
      <el-button @click="clearAll">↻ 清空全部</el-button>
    </div>

    <el-table
      :data="rows"
      border
      stripe
      style="width: 100%; margin-top: 12px;"
      @selection-change="selectedRows = $event"
      row-key="name"
    >
      <el-table-column type="selection" width="40" />
      <el-table-column label="参数名称" min-width="140">
        <template #default="{ row }">
          <el-input v-model="row.name" placeholder="参数名称" />
        </template>
      </el-table-column>
      <el-table-column label="别名" min-width="110">
        <template #default="{ row }">
          <el-input v-model="row.alias" placeholder="别名" />
        </template>
      </el-table-column>
      <el-table-column label="单位" width="90" align="center">
        <template #default="{ row }">
          <el-input v-model="row.unit" placeholder="单位" />
        </template>
      </el-table-column>
      <el-table-column label="数据类型" width="130" align="center">
        <template #default="{ row }">
          <el-select
            v-model="row.dataType"
            placeholder="选择类型"
            filterable
            style="width: 100%;"
          >
            <el-option
              v-for="t in CSharpTypes"
              :key="t.label"
              :label="t.label + (t.size > 0 ? ` (${t.size}B)` : ' (可变)')"
              :value="t.label"
            />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column label="备注" min-width="160">
        <template #default="{ row }">
          <el-input v-model="row.remark" placeholder="备注" />
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<style scoped>
.csv-editor {
  padding: 16px;
  max-width: 1200px;
  margin: 0 auto;
}

.header {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.title {
  font-size: 16px;
  font-weight: bold;
}

.tip {
  font-size: 12px;
  color: #999;
}

.toolbar {
  margin-top: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
</style>