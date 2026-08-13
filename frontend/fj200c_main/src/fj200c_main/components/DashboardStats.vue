<!--
  DashboardStats.vue — 仪表盘统计面板（fj200c_main 模块）

  显示发动机五大关键参数（Ng/Np 转速、排气温度、功率、燃油流量）的
  ECharts 仪表盘组件，以及 8 项环境/测功/流量参数的 el-table。

  注：大气温度/湿度/压力取 Adam4117 通道，进口温度取 Adam4015 通道。

  布局：
  - 左侧 el-card "仪表盘"：5 个 GaugeCard 横向排列
  - 右侧 el-card "环境参数"：el-table（无表头，标签+值 两列）
-->
<script lang="ts" setup>
import { useDashboardStore } from '../store/dashboard'
import GaugeCard from './GaugeCard.vue'

const store = useDashboardStore()

const gauges = [
  { key: 'ngSpeed' as const, label: 'Ng转速', unit: 'r/min', max: 15000 },
  { key: 'exhaustTemp' as const, label: '排气温度', unit: '℃', max: 1200 },
  { key: 'dynamometerPower' as const, label: '测功机功率', unit: 'kW', max: 50 },
  { key: 'fuelFlow' as const, label: '燃油流量', unit: 'L/h', max: 6000 },
  { key: 'npSpeed' as const, label: 'Np转速', unit: 'r/min', max: 15000 },
]
</script>

<template>
  <div class="dashboard-body">
    <el-card class="dashboard-card" shadow="never">
      <template #header>仪表盘</template>
      <div class="gauges-row">
        <GaugeCard
          v-for="g in gauges"
          :key="g.key"
          :label="g.label"
          :max="g.max"
          :unit="g.unit"
          :value="store.dashboardState[g.key]"
        />
      </div>
    </el-card>
    <el-card class="params-card" shadow="never">
      <template #header>环境参数</template>
      <el-table
        :data="store.envParams"
        :show-header="false"
        border
        class="params-table"
      >
        <el-table-column>
          <template #default="{ row }">
            <span class="param-label">{{ row.label }}{{ row.unit }}</span>
          </template>
        </el-table-column>
        <el-table-column>
          <template #default="{ row }">
            <span class="param-value">{{ Number(row.value).toFixed(1) }}</span>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<style scoped>
.dashboard-body {
  display: flex;
  gap: 12px;
}

.dashboard-card {
  flex: 1;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}

.dashboard-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}

.dashboard-card :deep(.el-card__body) {
  padding: 6px 0;
}

.gauges-row {
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  padding: 2px 0;
}

.params-card {
  width: 340px;
  flex-shrink: 0;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}

.params-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}

.params-card :deep(.el-card__body) {
  padding: 6px;
}

.params-table {
  width: 100%;
}

.params-table :deep(.el-table__cell) {
  padding: 6px 8px;
}

.param-label {
  font-family: 'Microsoft YaHei', sans-serif;
  font-weight: 700;
  font-size: 18px;
  color: var(--text-primary);
  white-space: nowrap;
  text-align: left;
  display: block;
  transition: color 0.3s;
}

.param-value {
  font-size: 25px;
  font-weight: 700;
  color: var(--text-accent);
  font-family: 'Microsoft YaHei', sans-serif;
  text-align: right;
  display: block;
  transition: color 0.3s;
}
</style>
