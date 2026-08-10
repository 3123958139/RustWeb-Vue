<!--
  ExperimentView.vue — 试验数据查看页面（fj200c_main 模块）

  展示 ECU 实时状态参数（大字网格）以及故障码 1/2 和附件状态的色块面板。

  适配说明（Tauri → Web）：
  - 原版通过 setInterval 轮询 commands.getPortSnapshot(0)
  - Web 版直接读取 dashboard store（由 WebSocket port_data 事件实时驱动）
  - 不再使用独立子窗口，改为应用内路由页面
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page view-page">
        <!-- 左侧：ECU 状态大字网格（4 列 × 6 行，撑满整列高度） -->
        <div class="ecu-panel">
          <div class="fm-panel-header">ECU 状态</div>
          <div class="ecu-grid">
            <div v-for="item in ecuItems" :key="item.key" class="ecu-cell">
              <span class="ecu-label">{{ item.label }}</span>
              <span class="ecu-value" :class="item.cls">{{ item.value }}</span>
            </div>
          </div>
        </div>

        <!-- 右侧：故障码 1/2 与附件状态色块，三等分纵向排列 -->
        <div class="right-panel">
          <div class="fm-panel">
            <div class="fm-panel-header">故障码 1</div>
            <div class="fault-grid">
              <div
                v-for="item in faultCode1Items"
                :key="item.key"
                class="fault-cell"
                :class="{ alarm: store.faultCodes[item.key] }"
              >{{ item.label }}</div>
            </div>
          </div>

          <div class="fm-panel">
            <div class="fm-panel-header">故障码 2</div>
            <div class="fault-grid fault-grid-3">
              <div
                v-for="item in faultCode2Items"
                :key="item.key"
                class="fault-cell"
                :class="{ alarm: store.faultCodes[item.key] }"
              >{{ item.label }}</div>
            </div>
          </div>

          <div class="fm-panel">
            <div class="fm-panel-header">附件状态</div>
            <div class="accessory-row">
              <div
                v-for="item in accessoryItems"
                :key="item.key"
                class="fault-cell"
                :class="{ active: Boolean(store.ecuData[item.key]) }"
              >{{ item.label }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { useDashboardStore } from "@/fj200c_main/store/dashboard";
import type { EcuFields, FaultCodeFlags } from "@shared/api/generated";
import ScaledPage from "@/fj200c_main/components/ScaledPage.vue";

const store = useDashboardStore();

interface ECUItem {
  key: string;
  label: string;
  value: string;
  cls?: string;
}

interface FaultItem {
  label: string;
  key: keyof FaultCodeFlags;
}

interface AccessoryItem {
  label: string;
  key: keyof EcuFields;
}

/** ECU 状态参数显示项（基于 store 实时计算） */
const ecuItems = computed<ECUItem[]>(() => {
  const e = store.ecuData;
  const n = (v: number | null | undefined, d: number) => (v ?? 0).toFixed(d);
  return [
    { key: "machNumber", label: "飞行马赫数回传", value: n(e.machNumber, 2) },
    { key: "altitude", label: "海拔高度回传", value: n(e.altitude, 0) },
    { key: "ngSpeed", label: "燃气发生器转速Ng", value: `${n(store.dashboardState.ngSpeed, 0)} r/min` },
    { key: "exhaustTemp", label: "排气温度", value: `${n(store.dashboardState.exhaustTemp, 1)} ℃` },
    { key: "intakeTemp", label: "进气温度", value: `${n(e.intakeTemp, 1)} ℃` },
    { key: "npSpeed", label: "动力涡轮转速Np", value: `${n(store.dashboardState.npSpeed, 0)} r/min` },
    { key: "throttle", label: "油门", value: n(e.throttle, 0) },
    { key: "engineStatus", label: "发动机状态", value: e.engineStatus || "—" },
    { key: "workingVoltage", label: "工作电压", value: `${n(e.workingVoltage, 1)} V` },
    { key: "cmdExecStatus", label: "控制指令执行情况", value: e.cmdExecStatus || "—" },
    { key: "faultCode1", label: "故障码1", value: String(e.faultCode1 ?? 0) },
    { key: "faultCode2", label: "故障码2", value: String(e.faultCode2 ?? 0) },
    { key: "oilPressure", label: "滑油压力", value: `${n(e.oilPressure, 1)} kPa` },
    { key: "oilTemp", label: "滑油温度", value: `${n(e.oilTemp, 1)} ℃` },
    { key: "fuelPressure", label: "燃油压力", value: `${n(e.fuelPressure, 0)} kPa` },
    { key: "accessoryStatus", label: "附件状态（hex）", value: e.accessoryStatus || "0" },
    { key: "exchangerOutletTemp", label: "换热器出口滑油温度", value: `${n(e.exchangerOutletTemp, 1)} ℃` },
    { key: "fingerprintCode", label: "指纹码", value: e.fingerprintCode || "—", cls: "mono" },
    { key: "frameCount", label: "帧计数", value: n(e.frameCount, 0) },
    { key: "engineStatusU8", label: "发动机状态（hex）", value: e.engineStatusU8 || "0" },
    { key: "cmdExecU8", label: "控制指令执行情况（hex）", value: e.cmdExecU8 || "0" },
  ];
});

const faultCode1Items: FaultItem[] = [
  { label: "自检排温异常", key: "fc1SelfCheckExhaust" },
  { label: "自检进温异常", key: "fc1SelfCheckIntakeTemp" },
  { label: "自检滑压异常", key: "fc1SelfCheckOilPressure" },
  { label: "自检滑温异常", key: "fc1SelfCheckOilTemp" },
  { label: "自检燃压异常", key: "fc1SelfCheckFuelPressure" },
  { label: "自检Ng转速异常", key: "fc1SelfCheckNgSpeed" },
  { label: "自检Np转速异常", key: "fc1SelfCheckNpSpeed" },
  { label: "油路排气异常", key: "fc1SelfCheckFuelVent" },
  { label: "冷运转异常", key: "fc1ColdStartAbnormal" },
  { label: "点火失败", key: "fc1IgnitionFailure" },
  { label: "起动超温", key: "fc1Overtemp" },
  { label: "起动超时", key: "fc1StartTimeout" },
  { label: "起发转速低", key: "fc1StartSpeedLow" },
  { label: "Ng转速超转", key: "fc1NgOverspeed" },
  { label: "Np转速超转", key: "fc1NpOverspeed" },
  { label: "排温超温", key: "fc1ExhaustOvertemp" },
];

const faultCode2Items: FaultItem[] = [
  { label: "Ng转速故障", key: "fc2NgSpeedFault" },
  { label: "Np转速故障", key: "fc2NpSpeedFault" },
  { label: "排温故障", key: "fc2ExhaustTempFault" },
  { label: "滑温故障", key: "fc2OilTempFault" },
  { label: "滑压故障", key: "fc2OilPressureFault" },
  { label: "燃压故障", key: "fc2FuelPressureFault" },
  { label: "ECU电压异常", key: "fc2VoltageAbnormal" },
  { label: "起动电压异常", key: "fc2StartVoltageAbnormal" },
  { label: "发电电压异常", key: "fc2GenVoltageAbnormal" },
  { label: "空中熄火", key: "fc2InFlightFlameout" },
  { label: "通信断开", key: "fc2CommDisconnected" },
];

const accessoryItems: AccessoryItem[] = [
  { label: "停车电磁阀", key: "stopSolenoid" },
  { label: "燃油泵", key: "fuelPump" },
  { label: "滑油泵", key: "oilPump" },
  { label: "起发电机", key: "starter" },
  { label: "轮载状态", key: "wheelLoadStatus" },
];
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

/* 试验信息查看页：左右各占一半铺满 1920×1080 舞台，内部不滚动 */
.view-page {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  overflow: hidden;
}

/* 左半边：ECU 状态面板，网格行高弹性填充 */
.ecu-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--fm-panel);
  border: 1px solid var(--fm-border);
  border-radius: 8px;
  overflow: hidden;
}

.ecu-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-template-rows: repeat(6, 1fr);
  gap: 4px;
  padding: 8px;
}

.ecu-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--fm-border);
  padding: 6px 8px;
  border-radius: 4px;
  background: var(--fm-panel-alt);
  min-height: 0;
  text-align: center;
}

.ecu-label {
  font-family: "Microsoft YaHei", sans-serif;
  font-size: 14px;
  line-height: 1.4;
  color: var(--fm-text-secondary);
  margin-bottom: 6px;
  text-align: center;
  max-width: 100%;
  overflow-wrap: break-word;
}

.ecu-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--fm-accent);
  font-family: "Microsoft YaHei", sans-serif;
  white-space: nowrap;
}

.ecu-value.mono {
  font-family: Consolas, monospace;
}

/* 右半边：故障码 1/2 与附件状态面板，纵向三等分 */
.right-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  min-width: 0;
}

.right-panel .fm-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin-bottom: 0;
}

.fault-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-template-rows: repeat(4, 1fr);
  gap: 3px;
  padding: 8px;
}

.fault-grid-3 {
  grid-template-rows: repeat(3, 1fr);
}

.accessory-row {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 3px;
  padding: 8px;
}

.accessory-row .fault-cell {
  flex: 1;
}

.fault-cell {
  border: 1px solid var(--fm-border);
  padding: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--fm-text);
  text-align: center;
  background: var(--fm-panel-alt);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: background 0.2s, color 0.2s;
  overflow: hidden;
}

.fault-cell.alarm {
  background: var(--fm-danger);
  color: #fff;
  font-weight: 700;
}

.fault-cell.active {
  background: var(--fm-success);
  color: #fff;
  font-weight: 700;
}
</style>
