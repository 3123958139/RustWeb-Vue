<!--
  FaultDisplay.vue — 故障码与附件状态（fj200c_main 模块）

  三个卡片横向排列：故障码 1（16 项）、故障码 2（11 项）、附件状态（5 项）。
  - alarm 类（红色）：故障位为 true 时高亮
  - active 类（绿色）：附件运行中时高亮
-->
<script setup lang="ts">
import { useDashboardStore } from '../store/dashboard'
import type { EcuFields, FaultCodeFlags } from '@shared/api/generated'

const store = useDashboardStore()

interface FaultItem {
  label: string
  key: keyof FaultCodeFlags
}

interface AccessoryItem {
  label: string
  key: keyof EcuFields
}

const faultCode1Items: FaultItem[] = [
  { label: '自检排温异常', key: 'fc1SelfCheckExhaust' },
  { label: '自检进温异常', key: 'fc1SelfCheckIntakeTemp' },
  { label: '自检滑压异常', key: 'fc1SelfCheckOilPressure' },
  { label: '自检滑温异常', key: 'fc1SelfCheckOilTemp' },
  { label: '自检燃压异常', key: 'fc1SelfCheckFuelPressure' },
  { label: '自检Ng转速异常', key: 'fc1SelfCheckNgSpeed' },
  { label: '自检Np转速异常', key: 'fc1SelfCheckNpSpeed' },
  { label: '油路排气异常', key: 'fc1SelfCheckFuelVent' },
  { label: '冷运转异常', key: 'fc1ColdStartAbnormal' },
  { label: '点火失败', key: 'fc1IgnitionFailure' },
  { label: '起动超温', key: 'fc1Overtemp' },
  { label: '起动超时', key: 'fc1StartTimeout' },
  { label: '起发转速低', key: 'fc1StartSpeedLow' },
  { label: 'Ng转速超转', key: 'fc1NgOverspeed' },
  { label: 'Np转速超转', key: 'fc1NpOverspeed' },
  { label: '排温超温', key: 'fc1ExhaustOvertemp' },
]

const faultCode2Items: FaultItem[] = [
  { label: 'Ng转速故障', key: 'fc2NgSpeedFault' },
  { label: 'Np转速故障', key: 'fc2NpSpeedFault' },
  { label: '排温故障', key: 'fc2ExhaustTempFault' },
  { label: '滑温故障', key: 'fc2OilTempFault' },
  { label: '滑压故障', key: 'fc2OilPressureFault' },
  { label: '燃压故障', key: 'fc2FuelPressureFault' },
  { label: 'ECU电压异常', key: 'fc2VoltageAbnormal' },
  { label: '起动电压异常', key: 'fc2StartVoltageAbnormal' },
  { label: '发电电压异常', key: 'fc2GenVoltageAbnormal' },
  { label: '空中熄火', key: 'fc2InFlightFlameout' },
  { label: '通信断开', key: 'fc2CommDisconnected' },
]

const accessoryItems: AccessoryItem[] = [
  { label: '停车电磁阀', key: 'stopSolenoid' },
  { label: '燃油泵', key: 'fuelPump' },
  { label: '滑油泵', key: 'oilPump' },
  { label: '起发电机', key: 'starter' },
  { label: '轮载状态', key: 'wheelLoadStatus' },
]
</script>

<template>
  <div class="fault-row">
    <el-card class="fault-card" shadow="never">
      <template #header>故障码1</template>
      <div class="fault-grid">
        <div
          v-for="item in faultCode1Items"
          :key="item.key"
          class="fault-cell"
          :class="{ alarm: store.faultCodes[item.key] }"
        >{{ item.label }}</div>
      </div>
    </el-card>
    <el-card class="fault-card" shadow="never">
      <template #header>故障码2</template>
      <div class="fault-grid">
        <div
          v-for="item in faultCode2Items"
          :key="item.key"
          class="fault-cell"
          :class="{ alarm: store.faultCodes[item.key] }"
        >{{ item.label }}</div>
      </div>
    </el-card>
    <el-card class="fault-card" shadow="never">
      <template #header>附件状态</template>
      <div class="accessory-row">
        <div
          v-for="item in accessoryItems"
          :key="item.key"
          class="fault-cell"
          :class="{ active: Boolean(store.ecuData[item.key]) }"
        >{{ item.label }}</div>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.fault-row {
  display: flex;
  gap: 4px;
  flex: 0 0 auto;
}
.fault-card {
  flex: 1;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  transition: background 0.3s, border-color 0.3s;
}
.fault-card :deep(.el-card__header) {
  padding: 6px 12px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card-header);
  transition: background 0.3s, border-color 0.3s, color 0.3s;
}
.fault-card :deep(.el-card__body) {
  padding: 6px;
}
.fault-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 2px;
}
.accessory-row {
  display: flex;
  gap: 2px;
}
.accessory-row .fault-cell {
  flex: 1;
}
.fault-cell {
  border: 1px solid var(--border-color);
  padding: 3px 6px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  text-align: center;
  background: var(--bg-cell);
  transition: background 0.3s, color 0.3s, border-color 0.3s;
}
.fault-cell.alarm {
  background: var(--bg-alarm);
  color: var(--text-danger);
  font-weight: 700;
}
.fault-cell.active {
  background: var(--bg-success);
  color: var(--text-success);
  font-weight: 700;
}
</style>
