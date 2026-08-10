<!--
  Monitor.vue — 主仪表盘页面（fj200c_main 模块）

  从 Tauri 源 App.vue 移植：以 1920×1080 设计尺寸进行 CSS scale 自适应缩放。
  组装仪表盘、ECU 状态、故障显示、图表、控制面板和状态栏六大区域。

  - useBackendPorts() 订阅 WebSocket 实时数据（共享连接，页面级引用）
  - useTheme() 初始化主题
  - useWindowScale() 计算 CSS scale 比例

  布局：
  screen-root (100vw × 100vh)
    └── scaled-stage (1920 × 1080, CSS scale)
         └── app-container (flex column)
              ├── main-content (flex: 1)
              │   ├── section-dashboard → DashboardStats
              │   ├── section-middle → ECUStatus + FaultDisplay
              │   └── section-chart → ChartPanel + ControlPanel
              └── section-status → StatusBar
-->
<script setup lang="ts">
import { useBackendPorts } from '@/fj200c_main/composables/useBackendPorts'
import { useWindowScale } from '@/fj200c_main/composables/useWindowScale'
import { useTheme } from '@/fj200c_main/composables/useTheme'
import DashboardStats from '@/fj200c_main/components/DashboardStats.vue'
import ECUStatus from '@/fj200c_main/components/ECUStatus.vue'
import FaultDisplay from '@/fj200c_main/components/FaultDisplay.vue'
import ChartPanel from '@/fj200c_main/components/ChartPanel.vue'
import ControlPanel from '@/fj200c_main/components/ControlPanel.vue'
import StatusBar from '@/fj200c_main/components/StatusBar.vue'

// 主仪表盘页面建立 WebSocket 实时数据引用（应用级连接由 App.vue 持有）
useBackendPorts()
useTheme()

const { scale, rootRef, DESIGN_W, DESIGN_H } = useWindowScale()
</script>

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
        <main class="main-content">
          <div class="section-dashboard"><DashboardStats /></div>
          <div class="section-middle"><ECUStatus /><FaultDisplay /></div>
          <div class="section-chart">
            <div class="chart-control-row">
              <ChartPanel />
              <ControlPanel />
            </div>
          </div>
        </main>
        <div class="section-status"><StatusBar /></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.screen-root {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--bg-page);
  transition: background 0.3s;
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
  gap: 4px;
  overflow: hidden;
}
.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  overflow-x: hidden;
  min-height: 0;
}
.section-dashboard {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  padding: 0 12px;
}
.section-middle {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 0 12px;
  background: var(--bg-page);
  transition: background 0.3s;
}
.section-chart {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.section-status {
  padding: 0 12px;
}
.chart-control-row {
  display: flex;
  gap: 12px;
  padding: 0 12px;
  flex: 1;
  min-height: 0;
}
</style>
