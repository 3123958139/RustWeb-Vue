<!--
  GenerateReport.vue — 报表生成页面（fj200c_main 模块）

  移植自 WPF WpfApplication.Report 的 4 表格报表布局。
  用户选择 CSV 文件后，调用后端进行数据处理（滤波、均值、标况换算、拟合），
  返回结果展示为四个表格：基本信息、性能数据、标准数据、设计点数据。

  适配说明（Tauri → Web）：
  - 原版通过 <input type="file"> 读取本地 CSV 文件，Web 版改为从后端 csv 目录选择
  - 原版通过 commands.generateReport() 调用 Rust 后端，Web 版改为 fj200cMainApi.generateReport()
  - 状态点默认值不再从 config.ini 读取，改为内置默认值
  - 打印功能：window.print() 打印页面本身，@media print 解除缩放舞台、表格铺满整页并自动跨页
    （与 fj200c 参考项目一致的打印方式）
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page">
        <div class="report-wrap">
          <div class="watermark">©{{ currentYear }} 7304厂</div>
          <!--          <div class="fm-panel-header report-header">报表生成</div>-->

          <div class="fm-panel report-toolbar">
            <div class="toolbar-row">
              <label class="tool-label">数据文件：</label>
              <el-select
                  v-model="selectedFile"
                  :loading="fileLoading"
                  class="file-select"
                  filterable
                  placeholder="请选择 CSV 数据文件"
              >
                <el-option
                    v-for="f in csvFiles"
                    :key="f"
                    :label="f"
                    :value="f"
                />
              </el-select>
              <el-button :loading="fileLoading" size="small" @click="loadFileList">刷新列表</el-button>
            </div>
            <div class="toolbar-row">
              <label class="tool-label">状态点：</label>
              <el-input
                  v-model="statePoints"
                  class="state-points-input"
                  placeholder="逗号分隔，如 30000,31000,32000,..."
              />
              <el-button
                  :disabled="generating || !selectedFile"
                  :loading="generating"
                  size="small"
                  type="primary"
                  @click="generateReport"
              >
                {{ generating ? '生成中...' : '生成报表' }}
              </el-button>
            </div>
          </div>

          <div v-if="error" class="error-bar">{{ error }}</div>

          <div v-if="report" class="report-body">
            <div class="report-title">FJ-200C涡桨发动机试车数据</div>

            <div class="section">
              <div class="section-title">一、试验基本信息</div>
              <table class="info-table">
                <tr>
                  <td class="info-label">发动机编号</td>
                  <td class="info-value"><input v-model="basicInfo[0]" class="info-input"/></td>
                  <td class="info-label">燃气发生器编号</td>
                  <td class="info-value"><input v-model="basicInfo[1]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">电控器编号</td>
                  <td class="info-value"><input v-model="basicInfo[2]" class="info-input"/></td>
                  <td class="info-label">转速传感器编号</td>
                  <td class="info-value"><input v-model="basicInfo[3]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">滑油温压一体传感器编号</td>
                  <td class="info-value"><input v-model="basicInfo[4]" class="info-input"/></td>
                  <td class="info-label">试验项目</td>
                  <td class="info-value"><input v-model="basicInfo[5]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">试验时间</td>
                  <td class="info-value"><input v-model="basicInfo[6]" class="info-input"/></td>
                  <td class="info-label">发动机状态</td>
                  <td class="info-value"><input v-model="basicInfo[7]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">冷灵活性</td>
                  <td class="info-value"><input v-model="basicInfo[8]" class="info-input"/></td>
                  <td class="info-label">试验指挥</td>
                  <td class="info-value"><input v-model="basicInfo[9]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">热灵活性</td>
                  <td class="info-value"><input v-model="basicInfo[10]" class="info-input"/></td>
                  <td class="info-label">操作人员</td>
                  <td class="info-value"><input v-model="basicInfo[11]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">试验序号</td>
                  <td class="info-value"><input v-model="basicInfo[12]" class="info-input"/></td>
                  <td class="info-label">检验人员</td>
                  <td class="info-value"><input v-model="basicInfo[13]" class="info-input"/></td>
                </tr>
                <tr>
                  <td class="info-label">停车原因</td>
                  <td class="info-value" colspan="3"><input v-model="basicInfo[14]" class="info-input"/></td>
                </tr>
              </table>
            </div>

            <div class="section">
              <div class="section-title">二、试验性能数据</div>
              <table class="data-table">
                <thead>
                <tr>
                  <th>转速<br/>r/min</th>
                  <th>推力<br/>N</th>
                  <th>排气温度<br/>°C</th>
                  <th>燃油流量<br/>g/s</th>
                  <th>大气温度<br/>°C</th>
                  <th>大气压力<br/>KPa</th>
                </tr>
                </thead>
                <tbody>
                <tr v-for="(row, i) in report.performanceData" :key="i">
                  <td>{{ row.speed }}</td>
                  <td>{{ row.thrust }}</td>
                  <td>{{ row.exhaustTemp }}</td>
                  <td>{{ row.fuelFlow }}</td>
                  <td>{{ row.ambientTemp }}</td>
                  <td>{{ row.ambientPressure }}</td>
                </tr>
                </tbody>
              </table>
            </div>

            <div class="section">
              <div class="section-title">三、试验标准数据</div>
              <table class="data-table">
                <thead>
                <tr>
                  <th>转速<br/>r/min</th>
                  <th>推力<br/>daN</th>
                  <th>排气温度<br/>°C</th>
                  <th>耗油率<br/>kg/(daN·h)</th>
                </tr>
                </thead>
                <tbody>
                <tr v-for="(row, i) in report.standardData" :key="i">
                  <td>{{ row.speed }}</td>
                  <td>{{ row.thrust }}</td>
                  <td>{{ row.exhaustTemp }}</td>
                  <td>{{ row.sfc }}</td>
                </tr>
                </tbody>
              </table>
            </div>

            <div class="section">
              <div class="section-title">四、设计点性能数据</div>
              <table class="data-table">
                <thead>
                <tr>
                  <th>转速<br/>r/min</th>
                  <th>推力<br/>daN</th>
                  <th>排气温度<br/>°C</th>
                  <th>耗油率<br/>kg/(daN·h)</th>
                </tr>
                </thead>
                <tbody>
                <tr v-for="(row, i) in report.designPointData" :key="i">
                  <td>{{ row.speed }}</td>
                  <td>{{ row.thrust }}</td>
                  <td>{{ row.exhaustTemp }}</td>
                  <td>{{ row.sfc }}</td>
                </tr>
                </tbody>
              </table>
            </div>

            <div class="print-area">
              <el-button type="primary" @click="handlePrint">打印 / 导出PDF</el-button>
            </div>
          </div>

          <div v-else-if="!generating" class="empty-state">
            请选择 CSV 数据文件并点击"生成报表"
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script lang="ts" setup>
import {onMounted, ref} from 'vue'
import {ElMessage} from 'element-plus'
import type {ReportOutput} from '@shared/api/generated'
import {fj200cMainApi} from '@/api'
import ScaledPage from '@/fj200c_main/components/ScaledPage.vue'

const currentYear = new Date().getFullYear()

/** 默认状态点（24 个，来自 config-fj200c_main.ini 的 [REPORT] StatePoints） */
const DEFAULT_STATE_POINTS = '30000,31000,32000,33000,34000,35000,36000,37000,38000,39000,40000,41000,42000,43000,44000,45000,46000,47000,48000,49000,50000,51000,52000,53000'

const selectedFile = ref('')
const csvFiles = ref<string[]>([])
const fileLoading = ref(false)
const statePoints = ref(DEFAULT_STATE_POINTS)

const basicInfo = ref<string[]>(Array(15).fill(''))
const generating = ref(false)
const error = ref('')
const report = ref<ReportOutput | null>(null)

/** 加载 CSV 文件列表 */
async function loadFileList() {
  fileLoading.value = true
  try {
    const response = await fj200cMainApi.listCsvFiles()
    if (response.success && response.data) {
      csvFiles.value = response.data.files
    } else {
      ElMessage.error(response.message || '加载文件列表失败')
    }
  } catch (err: any) {
    ElMessage.error(err.response?.data?.message || '加载文件列表失败')
  } finally {
    fileLoading.value = false
  }
}

/** 生成报表 */
async function generateReport() {
  if (!selectedFile.value) {
    error.value = '请先选择数据文件'
    return
  }
  if (!statePoints.value.trim()) {
    error.value = '请填写状态点'
    return
  }
  generating.value = true
  error.value = ''
  report.value = null
  try {
    // 先获取 CSV 文件内容
    const csvResp = await fj200cMainApi.getCsvFile(selectedFile.value)
    if (!csvResp.success || !csvResp.data) {
      error.value = csvResp.message || '读取 CSV 文件失败'
      return
    }
    // 调用后端生成报表
    const response = await fj200cMainApi.generateReport(
        selectedFile.value,
        csvResp.data.content,
        statePoints.value,
    )
    if (response.success && response.data) {
      report.value = response.data
      basicInfo.value = response.data.basicInfo.length === 15
          ? [...response.data.basicInfo]
          : Array(15).fill('')
    } else {
      error.value = response.message || '生成失败'
    }
  } catch (err: any) {
    error.value = err.response?.data?.message || '生成失败'
  } finally {
    generating.value = false
  }
}

/** 打印报表：window.print() 打印页面本身，由 @media print 样式控制只输出报表内容
 * （与 fj200c 参考项目一致的打印方式，表格铺满纸宽、自动跨页） */
function handlePrint() {
  if (!report.value) {
    ElMessage.warning('请先生成报表')
    return
  }
  window.print()
}

onMounted(() => {
  loadFileList()
})
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

.report-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--fm-panel);
  border: 1px solid var(--fm-border);
  border-radius: 8px;
}

.watermark {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  pointer-events: none;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 60pt;
  font-weight: bold;
  color: rgba(160, 170, 190, 0.15);
  transform: rotate(-30deg);
  font-family: 'Microsoft YaHei', sans-serif;
  white-space: nowrap;
  letter-spacing: 8pt;
}

.report-header {
  position: relative;
  z-index: 1;
}

.report-toolbar {
  position: relative;
  z-index: 1;
  margin: 16px;
  margin-bottom: 0;
}

.toolbar-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.toolbar-row:last-child {
  margin-bottom: 0;
}

.tool-label {
  color: var(--fm-text);
  font-size: 13px;
  white-space: nowrap;
  width: 80px;
  flex-shrink: 0;
}

.file-select {
  flex: 1;
  min-width: 0;
}

.state-points-input {
  flex: 1;
}

.error-bar {
  margin: 12px 16px 0;
  padding: 8px 16px;
  background: var(--fm-danger);
  color: #fff;
  font-size: 13px;
  border-radius: 4px;
}

.report-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  position: relative;
  z-index: 1;
}

.report-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fm-text);
  text-align: center;
  margin-bottom: 20px;
  padding-bottom: 10px;
  border-bottom: 2px solid var(--fm-border);
}

.section {
  margin-bottom: 20px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--fm-accent);
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--fm-border);
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.info-table td {
  padding: 4px 8px;
  border: 1px solid var(--fm-border);
  color: var(--fm-text);
}

.info-label {
  background: var(--fm-panel-alt);
  font-weight: 600;
  color: var(--fm-text-secondary) !important;
  width: 120px;
}

.info-value {
  background: var(--fm-panel);
  width: 200px;
}

.info-input {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--fm-text);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  padding: 0;
  box-sizing: border-box;
}

.info-input:focus {
  background: var(--fm-accent-soft);
}

.data-table th {
  background: var(--fm-panel-alt);
  color: var(--fm-accent);
  padding: 6px 8px;
  border: 1px solid var(--fm-border);
  font-weight: 600;
  text-align: center;
}

.data-table td {
  padding: 4px 8px;
  border: 1px solid var(--fm-border);
  color: var(--fm-text);
  text-align: center;
  background: var(--fm-panel);
}

.data-table tbody tr:hover td {
  background: var(--fm-accent-soft);
}

.print-area {
  text-align: center;
  padding: 20px 0;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--fm-text-secondary);
  font-size: 16px;
  padding: 40px;
}

@media print {
  .fj200c_main-root .fm-page {
    padding: 0;
    max-width: none;
  }

  .report-toolbar,
  .print-area,
  .error-bar,
  .empty-state {
    display: none !important;
  }

  .report-wrap {
    border: none !important;
    min-height: auto !important;
  }

  .report-body {
    overflow: visible !important;
    padding: 3mm 5mm !important;
  }

  .report-title {
    color: #222 !important;
    border-bottom-color: #333 !important;
  }

  .section-title {
    color: #333 !important;
    border-bottom-color: #999 !important;
  }

  table {
    page-break-inside: auto;
  }

  tr {
    page-break-inside: avoid;
  }

  thead {
    display: table-header-group;
  }

  .info-table td,
  .data-table td,
  .data-table th {
    color: #222 !important;
    background: #fff !important;
    border-color: #999 !important;
    word-break: break-word;
  }

  .info-label {
    background: #f5f5f5 !important;
    color: #333 !important;
    width: auto !important;
    max-width: 50%;
  }

  .info-value {
    width: auto !important;
  }

  .info-input {
    border: none !important;
    background: transparent !important;
    color: #222 !important;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }

  .data-table th {
    background: #e8e8e8 !important;
    color: #333 !important;
  }

  .watermark {
    display: flex !important;
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 9999;
    pointer-events: none;
    align-items: center;
    justify-content: center;
    font-size: 60pt;
    font-weight: bold;
    color: rgba(160, 170, 190, 0.15);
    transform: rotate(-30deg);
    font-family: 'Microsoft YaHei', sans-serif;
    white-space: nowrap;
    letter-spacing: 8pt;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
}
</style>

<style>
/* 全局打印样式（不能 scoped，需作用于 ScaledPage 缩放置舞台与 @page 纸张） */
@page {
  margin: 15mm;
}

@media print {
  /* 隐藏共享导航栏（不能 scoped，需作用于 packages/shared 的 AppNavbar） */
  .app-navbar-slot {
    display: none !important;
  }

  html,
  body {
    overflow: visible;
    height: auto;
  }

  /* 解除 1920×1080 缩放舞台：transform 缩放会把报表缩成纸上一小块（窄条） */
  .screen-root {
    overflow: visible !important;
    height: auto !important;
    justify-content: flex-start;
    align-items: flex-start;
  }

  .scaled-stage {
    width: 100% !important;
    height: auto !important;
    transform: none !important;
    overflow: visible !important;
  }
}
</style>
