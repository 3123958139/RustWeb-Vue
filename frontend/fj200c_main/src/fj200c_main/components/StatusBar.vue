<!--
  StatusBar.vue — 底部状态栏（fj200c_main 模块）

  显示五路串口通信统计：ECU 接收字节数/帧数、Adam4015/Adam4117/Dyno/Flux 接收字节数。
  以及最后发送的十六进制指令。模拟运行时显示"模拟运行中"徽章。
-->
<script setup lang="ts">
import { ref } from 'vue'
import { useDashboardStore } from '../store/dashboard'

const store = useDashboardStore()

const showHex = ref(false)

const statItems = [
  { label: 'ECU接收字节数', key: 'ecuRxBytes' as const },
  { label: 'ECU接收帧数', key: 'ecuRxFrames' as const },
  { label: 'Adam4015接收字节数', key: 'adam4015RxBytes' as const },
  { label: 'Adam4117接收字节数', key: 'adam4117RxBytes' as const },
  { label: 'Dyno接收字节数', key: 'dynoRxBytes' as const },
  { label: 'Flux接收字节数', key: 'fluxRxBytes' as const },
]
</script>

<template>
  <el-card shadow="never" class="status-card">
    <div class="status-bar">
      <div class="status-bar-left">
        <div v-if="store.isSimulating" class="sim-badge">模拟运行中</div>
        <div
          v-for="item in statItems"
          :key="item.key"
          class="stat-item"
        >
          <span class="stat-label">{{ item.label }}</span>
          <span class="stat-value">{{ store.footerStats[item.key] }}</span>
        </div>
      </div>
      <div class="status-bar-right">
        <el-button size="small" class="hex-btn" @click="showHex = !showHex">
          {{ showHex ? '隐藏发送帧' : '显示发送帧' }}
        </el-button>
        <span v-if="showHex" class="hex-value">
          <span v-if="store.footerStats.lastSentName" class="sent-name">{{ store.footerStats.lastSentName }}</span>
          {{ store.footerStats.lastSentHex || '无数据' }}
        </span>
      </div>
    </div>
  </el-card>
</template>

<style scoped>
.status-card {
  border: 1px solid var(--border-color);
  border-top: none;
  background: var(--bg-card);
  transition: background 0.3s, border-color 0.3s;
}
.status-card :deep(.el-card__body) {
  padding: 0;
}
.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  color: var(--text-secondary);
  font-size: 11px;
  flex-shrink: 0;
  transition: color 0.3s;
}
.status-bar-left {
  display: flex;
  gap: 20px;
  flex-wrap: wrap;
  align-items: center;
}
.status-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.sim-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 1px 10px;
  font-size: 11px;
  font-weight: 700;
  color: #fff;
  background: #e6a23c;
  border-radius: 10px;
  white-space: nowrap;
  animation: sim-pulse 2s ease-in-out infinite;
}
@keyframes sim-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
.stat-item {
  display: flex;
  gap: 4px;
  white-space: nowrap;
}
.stat-label {
  opacity: 0.8;
}
.stat-value {
  font-weight: 600;
  font-family: 'Microsoft YaHei', sans-serif;
  color: var(--text-accent);
  transition: color 0.3s;
}
.hex-btn {
  font-size: 11px;
  padding: 2px 8px;
  height: auto;
  line-height: 1.6;
}
.sent-name {
  color: var(--text-accent-green);
  margin-right: 6px;
  font-weight: 700;
  transition: color 0.3s;
}
.hex-value {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: var(--text-hex);
  font-weight: 600;
  max-width: 400px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: color 0.3s;
}
</style>
