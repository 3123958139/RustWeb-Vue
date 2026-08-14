<!--
  GaugeCard.vue — 仪表盘单个表盘（fj200c_main 模块）

  基于 ECharts Gauge 系列的圆形仪表盘，支持动画数值过渡、自适应缩放。
  接收四个 props（label/value/unit/max），适用于发动机转速/温度/功率等场景。

  - 颜色段通过 CSS 变量 --gauge-color-1..4 控制，主题切换时自动重绘
  - ResizeObserver 监听容器尺寸自动 resize
  - 同时监听 screen-resize / theme-changed 全局事件
-->
<script lang="ts" setup>
import {onMounted, onUnmounted, ref, watch} from 'vue'
import * as echarts from 'echarts'

const props = defineProps<{
  label: string
  value: number
  unit: string
  max: number
}>()

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null
let observer: ResizeObserver | null = null

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function getOption(): echarts.EChartsOption {
  return {
    series: [
      {
        type: 'gauge',
        startAngle: 220,
        endAngle: -40,
        min: 0,
        max: props.max,
        radius: '85%',
        center: ['50%', '55%'],
        axisLine: {
          lineStyle: {
            width: 8,
            color: [
              [0.25, cssVar('--gauge-color-1')],
              [0.5, cssVar('--gauge-color-2')],
              [0.75, cssVar('--gauge-color-3')],
              [1, cssVar('--gauge-color-4')],
            ],
          },
        },
        splitNumber: 4,
        axisTick: {
          length: 8,
          splitNumber: 5,
          lineStyle: {color: cssVar('--text-muted'), width: 1.5},
        },
        splitLine: {
          length: 16,
          lineStyle: {color: cssVar('--border-color'), width: 3},
        },
        axisLabel: {
          distance: 14,
          fontSize: 11,
          color: cssVar('--text-muted'),
          formatter: (v: number) => v >= 1000 ? (v / 1000).toFixed(1) + 'K' : String(v),
        },
        pointer: {
          length: '58%',
          width: 5,
          itemStyle: {color: cssVar('--text-accent')},
        },
        detail: {
          valueAnimation: true,
          formatter: (v: number) => {
            // 在此处调用你的逻辑
            // 例如：return formatValue(value, props.unit);
            // 或者直接写死逻辑：
            if (props.unit.includes('r/min')) return v.toFixed(0);
            if (props.unit.includes('kW')) return v.toFixed(1);
            return v.toFixed(2);
          },
          color: cssVar('--text-detail'),
          fontSize: 44,
          fontWeight: 'bolder',
          fontFamily: 'Microsoft YaHei, sans-serif',
          offsetCenter: [0, '72%'],
        },
        title: {
          offsetCenter: [0, '20%'],
          fontSize: 13,
          color: cssVar('--text-muted'),
        },
        data: [{value: props.value, name: props.unit}],
      },
    ],
  }
}

function initChart() {
  if (!chartRef.value) return
  chartInstance = echarts.init(chartRef.value)
  chartInstance.setOption(getOption())
}

function updateChart() {
  if (!chartInstance) return
  chartInstance.setOption({
    series: [{data: [{value: props.value}]}],
    animation: false,
  })
}

watch(() => props.value, updateChart)

function onThemeChanged() {
  if (chartInstance) chartInstance.setOption(getOption(), true)
}

function onScreenResize() {
  chartInstance?.resize()
}

onMounted(() => {
  initChart()
  observer = new ResizeObserver(() => chartInstance?.resize())
  if (chartRef.value) observer.observe(chartRef.value)
  window.addEventListener('theme-changed', onThemeChanged)
  window.addEventListener('screen-resize', onScreenResize)
})

onUnmounted(() => {
  observer?.disconnect()
  window.removeEventListener('theme-changed', onThemeChanged)
  window.removeEventListener('screen-resize', onScreenResize)
  chartInstance?.dispose()
})
</script>

<template>
  <div class="gauge-card">
    <div class="gauge-label">{{ label }}</div>
    <div ref="chartRef" class="gauge-chart"/>
  </div>
</template>

<style scoped>
.gauge-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  max-width: 280px;
}

.gauge-label {
  font-size: 26px;
  font-weight: 600;
  color: var(--text-primary);
  text-align: center;
  white-space: nowrap;
  transition: color 0.3s;
}

.gauge-chart {
  width: 260px;
  height: 260px;
}
</style>
