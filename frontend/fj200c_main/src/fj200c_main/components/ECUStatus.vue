<!--
  ECUStatus.vue — ECU 状态参数面板（fj200c_main 模块）

  显示发动机 ECU 的实时参数：马赫数、海拔、转速、温度、电压、压力等。
  所有数据来自 Pinia store 的 `ecuData`，由后端 WebSocket `port_data` 事件驱动更新。
-->
<script lang="ts" setup>
import { useDashboardStore } from '../store/dashboard'

const store = useDashboardStore()
const n = (v: number | null | undefined, d: number) => (v ?? 0).toFixed(d)
</script>

<template>
  <el-card class="middle-card" shadow="never">
    <template #header>ECU状态</template>
    <div class="ecu-params-grid">
      <div class="ecu-param-item">
        <span class="ep-label">飞行马赫数回传</span>
        <span class="ep-value">{{ n(store.ecuData.machNumber, 2) }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">海拔高度回传</span>
        <span class="ep-value">{{ n(store.ecuData.altitude, 0) }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">燃气发生器转速Ng</span>
        <span class="ep-value">{{ n(store.dashboardState.ngSpeed, 0) }} r/min</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">排气温度</span>
        <span class="ep-value">{{ n(store.dashboardState.exhaustTemp, 1) }} ℃</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">进气温度</span>
        <span class="ep-value">{{ n(store.ecuData.intakeTemp, 1) }} ℃</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">动力涡轮转速Np</span>
        <span class="ep-value">{{ n(store.dashboardState.npSpeed, 0) }} r/min</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">油门</span>
        <span class="ep-value">{{ n(store.ecuData.throttle, 0) }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">发动机状态</span>
        <span class="ep-value">{{ store.ecuData.engineStatus }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">工作电压</span>
        <span class="ep-value">{{ n(store.ecuData.workingVoltage, 1) }} V</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">控制指令执行情况</span>
        <span class="ep-value">{{ store.ecuData.cmdExecStatus }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">故障码1</span>
        <span class="ep-value">{{ store.ecuData.faultCode1 }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">故障码2</span>
        <span class="ep-value">{{ store.ecuData.faultCode2 }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">滑油压力</span>
        <span class="ep-value">{{ n(store.ecuData.oilPressure, 1) }} kPa</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">滑油温度</span>
        <span class="ep-value">{{ n(store.ecuData.oilTemp, 1) }} ℃</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">燃油压力</span>
        <span class="ep-value">{{ n(store.ecuData.fuelPressure, 0) }} kPa</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">附件状态（hex）</span>
        <span class="ep-value">{{ store.ecuData.accessoryStatus }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">换热器出口滑油温度</span>
        <span class="ep-value">{{ n(store.ecuData.exchangerOutletTemp, 1) }} ℃</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">指纹码</span>
        <span class="ep-value ep-fingerprint">{{ store.ecuData.fingerprintCode }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">帧计数</span>
        <span class="ep-value">{{ n(store.ecuData.frameCount, 0) }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">发动机状态（hex）</span>
        <span class="ep-value">{{ store.ecuData.engineStatusU8 }}</span>
      </div>
      <div class="ecu-param-item">
        <span class="ep-label">控制指令执行情况（hex）</span>
        <span class="ep-value">{{ store.ecuData.cmdExecU8 }}</span>
      </div>
    </div>
  </el-card>
</template>

<style scoped>
.middle-card {
  flex: 0 0 auto;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}

.middle-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}

.middle-card :deep(.el-card__body) {
  padding: 6px;
}

.ecu-params-grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 3px;
}

.ecu-param-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  border: 1px solid var(--border-color);
  padding: 4px 2px;
  background: var(--bg-cell);
  transition: background 0.3s, border-color 0.3s;
}

.ep-label {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  transition: color 0.3s;
}

.ep-value {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-accent);
  font-family: 'Microsoft YaHei', sans-serif;
  transition: color 0.3s;
}

.ep-fingerprint {
  font-family: Consolas, monospace;
}
</style>
