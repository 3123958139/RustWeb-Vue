<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { ProtocolField, CsvParameter } from '../types/protocol'
import { CSharpTypes, getTypeSize } from '../types/protocol'
import { recalcFields } from '../utils/protocol'
import { protocolGeneratorApi } from '@/api'

const router = useRouter()
const fields = ref<ProtocolField[]>([])
const selectedRows = ref<ProtocolField[]>([])
const markdownDialogVisible = ref(false)
const markdownContent = ref('')
const parameterCatalog = ref<CsvParameter[]>([])
const reportTitle = ref(localStorage.getItem('reportTitle') || '通信协议表')
const fileInput = ref<HTMLInputElement>()

function onTitleChange() {
  localStorage.setItem('reportTitle', reportTitle.value)
}

async function reloadCatalog() {
  try {
    const res = await protocolGeneratorApi.getDefaultCsv()
    parameterCatalog.value = res.data ?? []
  } catch (e: any) {
    ElMessage.error('加载参数表失败: ' + (e?.response?.data?.message || e))
  }
}

onMounted(async () => {
  await reloadCatalog()
})

function onNameChange(row: ProtocolField, name: string) {
  const p = parameterCatalog.value.find(x => x.name === name)
  if (p) {
    row.unit = p.unit
    row.dataType = p.dataType
    row.remark = p.remark
    onDataTypeChange()
  }
}

function addRow() {
  const newField: ProtocolField = {
    index: fields.value.length + 1,
    byteRange: '',
    name: '',
    unit: '',
    dataType: '',
    remark: '',
  }
  fields.value.push(newField)
}

function deleteSelected() {
  if (selectedRows.value.length === 0) {
    ElMessage.warning('请先选择要删除的行')
    return
  }
  const selected = new Set(selectedRows.value)
  fields.value = fields.value.filter(f => !selected.has(f))
  if (fields.value.length > 0) {
    fields.value = recalcFields(fields.value)
  }
}

function clearAll() {
  fields.value = []
}

function onDataTypeChange() {
  if (fields.value.length > 0) {
    fields.value = recalcFields(fields.value)
  }
}

function save() {
  const filename = (reportTitle.value || '通信协议表').replace(/[\\/:*?"<>|]/g, '_') + '.json'
  protocolGeneratorApi.downloadProtocolJson(fields.value, filename)
  ElMessage.success('已下载 JSON 文件')
}

function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  protocolGeneratorApi
    .parseProtocolJsonFile(file)
    .then(data => {
      fields.value = data
      ElMessage.success('加载成功')
    })
    .catch((e: any) => {
      ElMessage.error('加载失败: ' + e?.message)
    })
    .finally(() => {
      input.value = ''
    })
}

function load() {
  fileInput.value?.click()
}

async function exportMarkdown() {
  if (fields.value.length === 0) {
    ElMessage.warning('协议表为空，请先添加数据')
    return
  }
  try {
    const res = await protocolGeneratorApi.exportMarkdown({
      title: reportTitle.value,
      data: fields.value,
    })
    markdownContent.value = res.data?.content ?? ''
    markdownDialogVisible.value = true
  } catch (e: any) {
    ElMessage.error('导出失败: ' + (e?.response?.data?.message || e))
  }
}

async function copyMarkdown() {
  try {
    await navigator.clipboard.writeText(markdownContent.value)
    ElMessage.success('已复制到剪贴板')
  } catch {
    ElMessage.error('复制失败')
  }
}

async function exportExcel() {
  if (fields.value.length === 0) {
    ElMessage.warning('协议表为空，请先添加数据')
    return
  }
  try {
    const filename = (reportTitle.value || '通信协议表').replace(/[\\/:*?"<>|]/g, '_') + '.xlsx'
    await protocolGeneratorApi.exportExcel(
      { title: reportTitle.value, data: fields.value },
      filename,
    )
    ElMessage.success('导出成功')
  } catch (e: any) {
    ElMessage.error('导出失败: ' + e?.message)
  }
}

function printReport() {
  if (fields.value.length === 0) {
    ElMessage.warning('协议表为空，请先添加数据')
    return
  }

  const now = new Date().toLocaleDateString('zh-CN')
  const title = reportTitle.value || '通信协议表'
  const rows = fields.value.map(f => `
    <tr>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.index}</td>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.byteRange}</td>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.name}</td>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.unit}</td>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.dataType}</td>
      <td style="border:1px solid #000;padding:6px;text-align:center;">${f.remark}</td>
    </tr>`).join('')

  const html = `
    <div style="padding:30px;font-family:'Microsoft YaHei','SimSun',sans-serif;">
      <h1 style="text-align:center;font-size:20px;margin-bottom:4px;">${title}</h1>
      <p style="text-align:center;font-size:12px;color:#666;margin-bottom:20px;">${now}</p>
      <table style="width:100%;border-collapse:collapse;font-size:12px;">
        <thead>
          <tr style="background-color:#e8e8e8;">
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">序号</th>
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">字节范围</th>
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">参数名称</th>
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">单位</th>
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">数据类型</th>
            <th style="border:1px solid #000;padding:6px;text-align:center;font-weight:bold;">备注</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>`

  const container = document.createElement('div')
  container.innerHTML = html
  document.body.appendChild(container)

  const hiprintTemplate = new (window as any).hiprint.PrintTemplate()
  hiprintTemplate.printByHtml(container, {})

  setTimeout(() => document.body.removeChild(container), 2000)
}

function openCsvEditor() {
  router.push('/protocol_generator/csv')
}
</script>

<template>
  <div class="protocol-editor">
    <div class="toolbar">
      <span class="title-label">报表标题</span>
      <el-input
        v-model="reportTitle"
        style="width: 160px;"
        clearable
        placeholder="通信协议表"
        @change="onTitleChange"
      />
      <el-divider direction="vertical" />
      <el-button type="primary" @click="load">打开文件</el-button>
      <el-button type="success" @click="save">保存文件</el-button>
      <el-button @click="exportMarkdown">导出 Markdown</el-button>
      <el-button type="primary" @click="exportExcel">导出 Excel</el-button>
      <el-button type="warning" @click="printReport">打印报表</el-button>
      <el-divider direction="vertical" />
      <el-button @click="openCsvEditor">编辑 CSV</el-button>
    </div>

    <input
      ref="fileInput"
      type="file"
      accept=".json,application/json"
      style="display: none;"
      @change="onFileSelected"
    />

    <el-table
      :data="fields"
      border
      stripe
      style="width: 100%; margin-top: 12px;"
      @selection-change="selectedRows = $event"
      row-key="index"
    >
      <el-table-column type="selection" width="40" />
      <el-table-column label="序号" width="60" align="center">
        <template #default="{ row }">{{ row.index }}</template>
      </el-table-column>
      <el-table-column label="字节范围" width="100" align="center">
        <template #default="{ row }">{{ row.byteRange }}</template>
      </el-table-column>
      <el-table-column label="参数名称" min-width="140">
        <template #default="{ row }">
          <el-select
            v-model="row.name"
            placeholder="选择或输入参数名称"
            style="width: 100%;"
            filterable
            allow-create
            clearable
            @change="(val: string) => onNameChange(row, val)"
          >
            <el-option
              v-for="p in parameterCatalog"
              :key="p.name"
              :label="p.alias ? `${p.name}（${p.alias}）` : p.name"
              :value="p.name"
            />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column label="单位" width="80" align="center">
        <template #default="{ row }">
          <el-input v-model="row.unit" placeholder="单位" />
        </template>
      </el-table-column>
      <el-table-column label="数据类型" width="200" align="center">
        <template #default="{ row }">
          <el-select
            v-model="row.dataType"
            placeholder="选择类型"
            style="width: 100%;"
            @change="onDataTypeChange"
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
      <el-table-column label="长度" width="110" align="center">
        <template #default="{ row }">
          <el-input-number
            v-if="getTypeSize(row.dataType) === 0"
            v-model="row.length"
            :min="1"
            :max="65535"
            :controls="false"
            style="width: 100%;"
            placeholder="字节数"
            @change="onDataTypeChange"
          />
          <span v-else class="fixed-size">
            {{ row.dataType ? `${getTypeSize(row.dataType)} B` : '' }}
          </span>
        </template>
      </el-table-column>
      <el-table-column label="备注" min-width="140">
        <template #default="{ row }">
          <el-input v-model="row.remark" placeholder="备注" />
        </template>
      </el-table-column>
    </el-table>

    <div class="actions">
      <el-button type="primary" @click="addRow">+ 新增行</el-button>
      <el-button type="danger" @click="deleteSelected">× 删除选中</el-button>
      <el-button @click="clearAll">↻ 清空全部</el-button>
    </div>

    <el-dialog v-model="markdownDialogVisible" title="Markdown 预览" width="700px">
      <el-input
        type="textarea"
        :rows="15"
        :model-value="markdownContent"
        readonly
      />
      <template #footer>
        <el-button @click="copyMarkdown">复制到剪贴板</el-button>
        <el-button type="primary" @click="markdownDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.protocol-editor {
  padding: 16px;
  max-width: 1200px;
  margin: 0 auto;
}

.toolbar {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.title-label {
  font-size: 13px;
  color: #606266;
}

.fixed-size {
  font-size: 12px;
  color: #909399;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}
</style>