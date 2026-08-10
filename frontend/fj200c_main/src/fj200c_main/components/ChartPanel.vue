<!--
  ChartPanel.vue — 参数曲线图表（fj200c_main 模块）

  基于 ECharts 的多轴折线图，展示 Ng 转速、排气温度、测功机功率等历史趋势。
  左 Y 轴为转速，右 Y 轴为温度/功率/流量，每 1 秒新增一个数据点。

  - watch store.chartData 深度更新数据
  - 1 秒定时调用 store.addChartPoint()
  - 监听 screen-resize / theme-changed 事件
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { useDashboardStore } from '../store/dashboard'

// 按需注册图表模块，避免全量 echarts（1107KB）进入产物
echarts.use([LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer])

const store = useDashboardStore()
const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null

const seriesMeta = [
  { name: 'Ng转速', key: 'ngSpeed' as const, color: '#4fc3f7', yAxisIndex: 0 },
  { name: 'Np转速', key: 'npSpeed' as const, color: '#9575cd', yAxisIndex: 0 },
  { name: '排气温度', key: 'exhaustTemp' as const, color: '#ef5350', yAxisIndex: 1 },
  { name: '测功机功率', key: 'dynamometerPower' as const, color: '#ffb74d', yAxisIndex: 1 },
  { name: '燃油流量', key: 'fuelFlow' as const, color: '#4db6ac', yAxisIndex: 1 },
]

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function getChartOption() {
  return {
    grid: { left: 55, right: 65, top: 40, bottom: 30 },
    xAxis: {
      type: 'category',
      data: store.chartTime,
      axisLine: { lineStyle: { color: cssVar('--border-color') } },
      axisLabel: { fontSize: 10, color: cssVar('--text-muted') },
      splitLine: { show: false },
    },
    legend: {
      data: seriesMeta.map(s => s.name),
      top: 4,
      left: 'center',
      textStyle: { fontSize: 11, color: cssVar('--text-primary') },
      icon: 'roundRect',
      itemWidth: 14,
      itemHeight: 4,
    },
    yAxis: [
      {
        type: 'value',
        name: '转速 (r/min)',
        nameTextStyle: { color: cssVar('--text-muted'), fontSize: 10 },
        axisLabel: { fontSize: 10, color: cssVar('--text-muted') },
        splitLine: { lineStyle: { color: cssVar('--border-color'), type: 'dashed' } },
      },
      {
        type: 'value',
        name: '温度 / 功率 / 流量',
        nameTextStyle: { color: cssVar('--text-muted'), fontSize: 10 },
        axisLabel: { fontSize: 10, color: cssVar('--text-muted') },
        splitLine: { show: false },
      },
    ],
    series: seriesMeta.map(s => ({
      name: s.name,
      type: 'line' as const,
      data: store.chartData.map(d => d[s.key]),
      yAxisIndex: s.yAxisIndex,
      smooth: true,
      symbol: 'none',
      color: s.color,
      lineStyle: { width: 2 },
    })),
    tooltip: {
      trigger: 'axis',
      backgroundColor: cssVar('--bg-tooltip'),
      borderColor: cssVar('--border-color'),
      textStyle: { color: cssVar('--text-primary'), fontSize: 11 },
      formatter: (params: { dataIndex: number }[]) => {
        const idx = params[0].dataIndex
        const p = store.chartData[idx]
        if (!p) return ''
        let html = `${store.chartTime[idx]}<br/>`
        for (const s of seriesMeta) {
          html += `<span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${s.color};margin-right:4px"></span>${s.name}: ${p[s.key]}<br/>`
        }
        return html
      },
    },
  }
}

let timer: ReturnType<typeof setInterval> | undefined

function initChart() {
  if (!chartRef.value) return
  chartInstance = echarts.init(chartRef.value)
  chartInstance.setOption(getChartOption())
}

function updateChart() {
  if (!chartInstance) return
  chartInstance.setOption({
    xAxis: { data: store.chartTime },
    series: seriesMeta.map(s => ({
      data: store.chartData.map(d => d[s.key]),
    })),
    animation: false,
  })
}

watch(() => store.chartData, updateChart, { deep: true })

function onScreenResize() {
  chartInstance?.resize()
}

function onThemeChanged() {
  if (chartInstance) chartInstance.setOption(getChartOption(), true)
}

onMounted(() => {
  initChart()
  timer = setInterval(() => {
    store.addChartPoint()
  }, 1000)
  window.addEventListener('screen-resize', onScreenResize)
  window.addEventListener('theme-changed', onThemeChanged)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
  window.removeEventListener('screen-resize', onScreenResize)
  window.removeEventListener('theme-changed', onThemeChanged)
  chartInstance?.dispose()
})
</script>

<template>
  <el-card shadow="never" class="chart-card">
    <template #header>参数曲线</template>
    <div ref="chartRef" class="chart-container" />
  </el-card>
</template>

<style scoped>
.chart-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}
.chart-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}
.chart-card :deep(.el-card__body) {
  flex: 1;
  display: flex;
  padding: 0;
  min-height: 0;
  overflow: hidden;
}
.chart-container {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>
