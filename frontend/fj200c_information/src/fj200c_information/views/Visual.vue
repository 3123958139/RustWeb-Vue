<!--
  发动机可视化分析页

  使用 ECharts 仪表盘和折线图展示实时发动机数据：
  - 6 个仪表盘：Ng 转速、Np 转速、排气温度、滑油压力、油门、进气温度
  - 1 条实时曲线：所有参数的历史趋势
  数据通过 WebSocket 接收 frame 事件驱动更新。
-->
<template>
  <div class="fj200c_information-root">
    <div class="fj200c_information-page">
      <div class="fj200c_information-toolbar">
        <span class="toolbar-title">可视化分析</span>
        <el-tag :type="events.connected ? 'success' : 'warning'" size="small">
          {{ events.connected ? "数据连接正常" : "数据连接中…" }}
        </el-tag>
        <div class="spacer"></div>
        <ServiceNavButton />
        <el-button size="small" @click="clearCharts">清空曲线</el-button>
      </div>

      <!-- 6 个仪表盘（动态渲染） -->
      <div class="fj200c_information-gauge-grid">
        <div v-for="(item, i) in gaugeItems" :key="item.key" class="fj200c_information-chart-panel">
          <div class="chart-title">{{ item.name }}</div>
          <!--
            函数式 ref：
            Vue 3 的 :ref 可以绑定一个函数，组件挂载时调用该函数并传入 DOM 元素。
            这里用 setGaugeRef 将每个仪表盘 DOM 元素存入 gaugeRefs 数组。
          -->
          <div :ref="(el: unknown) => setGaugeRef(i, el)" class="fj200c_information-gauge-item"></div>
        </div>
      </div>

      <!-- 实时折线图 -->
      <div class="fj200c_information-chart-panel">
        <div class="chart-title">实时曲线</div>
        <div ref="lineRef" class="fj200c_information-vis-line"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import * as echarts from "echarts/core";
import { GaugeChart, LineChart } from "echarts/charts";
import { GridComponent, LegendComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import ServiceNavButton from "@/fj200c_information/components/ServiceNavButton.vue";
import { useFj200cInformationEvents } from "@/fj200c_information/composables/useFj200cInformationEvents";

// 按需注册图表模块，避免全量 echarts（1107KB）进入产物
echarts.use([GaugeChart, LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

/**
 * 仪表盘配置项
 *
 * key:   唯一标识（用于数据存储）
 * name:  显示名称
 * field: 服务端解码字段索引（见后端 decode.rs / CSV_HEADERS）
 * min:   最小值
 * max:   最大值
 * unit:  单位后缀
 * decimals: 小数位数
 */
interface GaugeItem {
  key: string;
  name: string;
  field: number;
  min: number;
  max: number;
  unit: string;
  decimals: number;
}

/** 6 个仪表盘配置 */
const gaugeItems: GaugeItem[] = [
  { key: "ng", name: "Ng 转速", field: 4, min: 0, max: 50000, unit: " r/min", decimals: 0 },
  { key: "np", name: "Np 转速", field: 7, min: 0, max: 50000, unit: " r/min", decimals: 0 },
  { key: "exhaust", name: "排气温度", field: 5, min: 0, max: 20000, unit: " °C", decimals: 0 },
  { key: "oilp", name: "滑油压力", field: 14, min: 0, max: 3, unit: " MPa", decimals: 2 },
  { key: "throttle", name: "油门", field: 8, min: 0, max: 500, unit: "", decimals: 2 },
  { key: "intake", name: "进气温度", field: 6, min: 0, max: 2000, unit: " °C", decimals: 1 },
];

/** 仪表盘配色方案 */
const PALETTE = ["#409eff", "#67c23a", "#e6a23c", "#f56c6c", "#8e5cd9", "#00b4d8"];
/** 折线图最大数据点数 */
const MAX_POINTS = 100;

/** 仪表盘 DOM 元素引用数组 */
const gaugeRefs: (HTMLElement | null)[] = [];
/** 折线图 DOM 元素引用 */
const lineRef = ref<HTMLElement | null>(null);
/** ECharts 图表实例数组 */
let gaugeCharts: echarts.ECharts[] = [];
/** 折线图 ECharts 实例 */
let lineChart: echarts.ECharts | null = null;
/** 各参数的时序数据（时间 + 值） */
const seriesData: Record<string, { time: string[]; value: number[] }> = {};

// 初始化每个参数的数据存储
for (const item of gaugeItems) {
  seriesData[item.key] = { time: [], value: [] };
}

/**
 * 函数式 ref：Vue 3 模板 ref 的函数形式
 * 当元素挂载/卸载时 Vue 会自动调用此函数
 */
const setGaugeRef = (index: number, el: unknown) => {
  gaugeRefs[index] = el as HTMLElement;
};

/** 初始化 6 个 ECharts 仪表盘 */
const initGauges = () => {
  gaugeItems.forEach((item, i) => {
    const el = gaugeRefs[i];
    if (!el) return;
    const chart = echarts.init(el);
    chart.setOption({
      backgroundColor: "transparent",
      series: [
        {
          type: "gauge",
          center: ["50%", "58%"],
          radius: "90%",
          startAngle: 210,
          endAngle: -30,
          min: item.min,
          max: item.max,
          splitNumber: 5,
          progress: { show: true, width: 12, roundCap: true, itemStyle: { color: PALETTE[i % PALETTE.length] } },
          axisLine: { lineStyle: { width: 12, color: [[1, "rgba(64,158,255,0.15)"]] } },
          axisTick: { distance: -14, length: 5, lineStyle: { color: "#909399", width: 1 } },
          splitLine: { distance: -14, length: 10, lineStyle: { color: "#909399", width: 2 } },
          axisLabel: { distance: -6, color: "#909399", fontSize: 9 },
          pointer: { width: 4, length: "60%", itemStyle: { color: PALETTE[i % PALETTE.length] } },
          anchor: { show: true, size: 8, itemStyle: { borderColor: PALETTE[i % PALETTE.length], borderWidth: 2 } },
          title: { offsetCenter: ["0%", "78%"], fontSize: 11, color: "#909399" },
          detail: {
            valueAnimation: true,
            fontSize: 13,
            offsetCenter: ["0%", "52%"],
            color: "#c8d6e5",
            formatter: (value: number) => value.toFixed(item.decimals) + item.unit,
          },
          data: [{ value: 0, name: item.name }],
        },
      ],
    });
    gaugeCharts.push(chart);
  });
};

/** 初始化折线图 */
const initLine = () => {
  if (!lineRef.value) return;
  lineChart = echarts.init(lineRef.value);
  lineChart.setOption({
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: { data: gaugeItems.map((item) => item.name), top: 0, textStyle: { color: "#909399", fontSize: 10 } },
    grid: { left: 56, right: 56, top: 32, bottom: 28 },
    xAxis: {
      type: "category",
      data: [],
      boundaryGap: false,
      axisLine: { lineStyle: { color: "#3a4a63" } },
      axisLabel: { color: "#909399", fontSize: 10 },
    },
    yAxis: [
      {
        type: "value",
        name: "转速 / 温度",
        splitLine: { lineStyle: { color: "rgba(200,214,229,0.12)" } },
        axisLabel: { color: "#909399", fontSize: 10 },
      },
      {
        type: "value",
        name: "压力 / 油门",
        splitLine: { show: false },
        axisLabel: { color: "#909399", fontSize: 10 },
      },
    ],
    // 每个参数对应一条折线，滑油压力和油门使用右侧 Y 轴
    series: gaugeItems.map((item, i) => ({
      name: item.name,
      type: "line",
      yAxisIndex: item.key === "oilp" || item.key === "throttle" ? 1 : 0,
      showSymbol: false,
      smooth: true,
      lineStyle: { width: 1.5, color: PALETTE[i % PALETTE.length] },
      itemStyle: { color: PALETTE[i % PALETTE.length] },
      data: [],
    })),
  });
};

/**
 * 帧事件回调：每收到一帧解码数据时更新图表
 *
 * @param fields - 服务端解码后的字段数组（按 CSV_HEADERS 索引）
 */
const onFrame = (fields: string[]) => {
  const label = fields[1] || ""; // 帧计数字段作为 X 轴标签

  // 更新每个仪表盘
  gaugeItems.forEach((item, i) => {
    const chart = gaugeCharts[i];
    if (!chart) return;
    const value = parseFloat(fields[item.field] || "");
    chart.setOption({
      series: [{ data: [{ value: isNaN(value) ? 0 : value, name: item.name }] }],
    });

    // 追加数据点到时序数据
    const series = seriesData[item.key];
    if (!isNaN(value)) {
      series.time.push(label);
      series.value.push(value);
      if (series.time.length > MAX_POINTS) {
        series.time.shift();   // 移除最早的数据点
        series.value.shift();
      }
    }
  });

  // 更新折线图
  if (!lineChart) return;
  lineChart.setOption({
    xAxis: { data: seriesData[gaugeItems[0].key].time },
    series: gaugeItems.map((item) => ({ data: seriesData[item.key].value })),
  });
};

/** 清空所有图表数据 */
const clearCharts = () => {
  for (const key of Object.keys(seriesData)) {
    seriesData[key] = { time: [], value: [] };
  }
  lineChart?.setOption({
    xAxis: { data: [] },
    series: gaugeItems.map(() => ({ data: [] })),
  });
};

/** 窗口大小变化时重新调整图表尺寸 */
const handleResize = () => {
  gaugeCharts.forEach((chart) => chart.resize());
  lineChart?.resize();
};

/** 创建 WebSocket 事件流连接 */
const events = useFj200cInformationEvents({ onFrame });
events.connect();

onMounted(() => {
  initGauges();
  initLine();
  window.addEventListener("resize", handleResize);
});

/**
 * 组件卸载时清理资源
 *
 * onUnmounted 是 Vue 3 生命周期钩子，在组件销毁后执行。
 * 必须移除事件监听器并销毁 ECharts 实例，防止内存泄漏。
 */
onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  gaugeCharts.forEach((chart) => chart.dispose()); // dispose() 销毁 ECharts 实例
  lineChart?.dispose();
});
</script>

<style scoped>
@import "@/fj200c_information/fj200c_information.css";

.spacer {
  flex: 1;
}
</style>
