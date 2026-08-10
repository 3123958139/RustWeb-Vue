<!--
  ControlPanel.vue — 控制面板（fj200c_main 模块）

  发动机远程控制面板，支持参数设置（马赫数、海拔、油门、轮载）和
  指令操作（起动、停车、冷运转、油路排气、恒定油门设定等）。
  通过 HTTP API `/api/fj200c_main/service/command` 发送十六进制指令到 ECU。

  - 帧头 EB 90 10，帧尾累加和校验（字节 0~14 的累加和，写入 frame[15]）
  - frame[3] 为帧序号（每次发送递增）
  - 按钮点击后构造完整帧并调用 fj200cMainApi.sendCommand()
-->
<script lang="ts" setup>
import { useDashboardStore } from '../store/dashboard'
import { fj200cMainApi } from '@/api'
import { ElMessage } from 'element-plus'

const store = useDashboardStore()

const CMD_BYTE: Record<string, number> = {
  '空中起动': 0x91,
  '起动': 0xA1,
  '燃气发生器冷运转': 0xB1,
  '停止燃气发生器冷运转': 0xC1,
  '恒定油门设定': 0xD1,
  '停车': 0xE1,
  '油路排气': 0xF1,
  '空白': 0x00,
  '电控器自检': 0x10,
}

let frameSeq = 0

function buildHex(data: number[]): string {
  return data.map(b => b.toString(16).padStart(2, '0').toUpperCase()).join('')
}

function buildBaseFrame(cmd: number): number[] {
  const MachNumber = Math.min(100, Math.max(0, Math.round(store.controlPanel.machNumber * 100)))
  const Altitude = Math.min(8000, Math.max(0, Math.round(store.controlPanel.altitude)))
  const ThrottleDuty = cmd === 0xD1 ? Math.round(store.controlPanel.throttleDuty * 100) : 0
  const WheelLoad = store.controlPanel.wheelLoad === '1' ? 1 : 0
  return [
    0xEB, 0x90, 0x10, 0,
    MachNumber,
    WheelLoad,
    Altitude & 0xFF, (Altitude >> 8) & 0xFF,
    0,
    cmd,
    ThrottleDuty & 0xFF, (ThrottleDuty >> 8) & 0xFF,
    0, 0, 0, 0,
  ]
}

async function sendConfig(frame: number[], name: string = ''): Promise<boolean> {
  let sum = 0
  for (let i = 0; i < 15; i++) {
    sum += frame[i]
  }
  frame[15] = sum & 0xFF
  frame[3] = frameSeq
  frameSeq = (frameSeq + 1) & 0xFF
  const hex = buildHex(frame)
  store.footerStats.lastSentHex = hex
  store.footerStats.lastSentName = name
  try {
    const response = await fj200cMainApi.sendCommand(hex)
    if (!response.success) {
      ElMessage.error(response.message || '发送失败')
      return false
    }
    return true
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '发送失败')
    return false
  }
}

function sendMachNumber() {
  sendConfig(buildBaseFrame(0x00), '飞行马赫数')
}

function sendAltitude() {
  sendConfig(buildBaseFrame(0x00), '海拔高度')
}

function sendThrottleDuty() {
  sendConfig(buildBaseFrame(0x00), '恒定油门占空比')
}

function sendWheelLoad() {
  sendConfig(buildBaseFrame(0x00), '轮载')
}

let recordingBusy = false

/** 联动 CSV 录制：目标状态与当前不一致时才切换（防止重复点击误翻转） */
async function syncRecording(target: boolean) {
  if (recordingBusy || store.isRecording === target) return
  recordingBusy = true
  try {
    const response = await fj200cMainApi.toggleRecording()
    if (response.success && response.data) {
      store.isRecording = response.data.recording
      ElMessage.success(store.isRecording ? '指令已发送，自动开始保存数据' : '指令已发送，自动停止保存数据')
    } else {
      ElMessage.error(response.message || '切换保存状态失败')
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '切换保存状态失败')
  } finally {
    recordingBusy = false
  }
}

async function sendCommand(label: string) {
  const cmdByte = CMD_BYTE[label]
  if (cmdByte === undefined) return
  const ok = await sendConfig(buildBaseFrame(cmdByte), label)
  if (!ok) return
  // 起动自动开始保存数据，停车自动停止保存（指令发送成功后才联动）
  if (label === '起动') await syncRecording(true)
  else if (label === '停车') await syncRecording(false)
}

const controlButtons = [
  { label: '起动', row: 0, col: 0 },
  { label: '停车', row: 0, col: 1 },
  { label: '燃气发生器冷运转', row: 0, col: 2 },
  { label: '油路排气', row: 0, col: 3 },
  { label: '停止燃气发生器冷运转', row: 0, col: 5 },
  { label: '空中起动', row: 1, col: 0 },
  { label: '空白', row: 1, col: 1 },
  { label: '恒定油门设定', row: 1, col: 2 },
  { label: '电控器自检', row: 1, col: 3 },
]
</script>

<template>
  <div class="control-panel">
    <el-card class="ctrl-card" shadow="never">
      <template #header>控制指令</template>
      <div class="control-config">
        <div class="config-row">
          <span class="config-label">飞行马赫数</span>
          <el-input v-model.number="store.controlPanel.machNumber" class="config-input" size="small" />
          <el-button size="small" @click="sendMachNumber()">确定</el-button>
        </div>
        <div class="config-row">
          <span class="config-label">海拔高度</span>
          <el-input v-model.number="store.controlPanel.altitude" class="config-input" size="small" />
          <el-button size="small" @click="sendAltitude()">确定</el-button>
        </div>
        <div class="config-row">
          <span class="config-label">恒定油门占空比</span>
          <el-input v-model.number="store.controlPanel.throttleDuty" class="config-input" size="small" />
          <el-button size="small" @click="sendThrottleDuty()">确定</el-button>
        </div>
        <div class="config-row">
          <span class="config-label">轮载</span>
          <el-select v-model="store.controlPanel.wheelLoad" class="config-select" size="small">
            <el-option label="地面" value="0" />
            <el-option label="空中" value="1" />
          </el-select>
          <el-button size="small" @click="sendWheelLoad()">确定</el-button>
        </div>
      </div>
      <div class="control-divider" />
      <div class="control-grid">
        <button
          v-for="btn in controlButtons"
          :key="btn.label"
          class="ctrl-btn"
          @click="sendCommand(btn.label)"
        >
          {{ btn.label }}
        </button>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.control-panel {
  width: 340px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ctrl-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}

.ctrl-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}

.ctrl-card :deep(.el-card__body) {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 6px;
}

.control-config {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.config-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
}

.config-label {
  width: 100px;
  flex-shrink: 0;
  color: var(--text-primary);
  font-weight: 500;
  transition: color 0.3s;
}

.config-input {
  flex: 1;
  min-width: 0;
}

.config-select {
  flex: 1;
  min-width: 0;
}

.config-row :deep(.el-button) {
  flex-shrink: 0;
}

.control-divider {
  height: 1px;
  background: var(--border-color);
  margin: 6px 0;
  transition: background 0.3s;
}

.control-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  grid-template-rows: 1fr 1fr;
  gap: 4px;
}

.ctrl-btn {
  padding: 4px;
  font-size: 14px;
  line-height: 1.2;
  background: var(--btn-bg);
  border: 1px solid var(--btn-border);
  border-radius: 3px;
  color: var(--btn-text);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  white-space: pre-line;
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
}

.ctrl-btn:hover {
  background: var(--btn-hover-bg);
  border-color: var(--btn-hover-border);
  color: var(--btn-hover-text);
}

.ctrl-btn:active {
  background: var(--btn-active-bg);
  border-color: var(--btn-active-border);
}
</style>
