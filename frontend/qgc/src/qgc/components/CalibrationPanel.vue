<!--
  传感器校准面板（qgc CalibrationPanel）

  发送 PREFLIGHT_CALIBRATION 命令（COMMAND_LONG, command=241）触发飞控校准流程。
  模拟器无真实传感器，回执为 UNSUPPORTED（需真实飞控）。本面板提供向导式入口与说明。
-->
<template>
  <div class="cal-panel">
    <el-steps :active="step" align-center finish-status="success" class="cal-steps">
      <el-step title="静止水平" />
      <el-step title="旋转各轴" />
      <el-step title="完成" />
    </el-steps>
    <div class="cal-type">
      <span class="cal-label">校准类型</span>
      <el-select v-model="calType" size="small" class="cal-select">
        <el-option label="陀螺仪" :value="1" />
        <el-option label="磁力计" :value="2" />
        <el-option label="加速度计" :value="3" />
        <el-option label="水平（六面）" :value="4" />
        <el-option label="加速度计+磁力计" :value="5" />
      </el-select>
    </div>
    <el-button type="primary" size="small" :loading="sending" @click="startCal">开始校准</el-button>
    <div class="cal-hint">
      <p>校准命令经 <code>sendCommand("calibrate", null, [类型])</code> 下发。</p>
      <p class="cal-warn">模拟器无真实 IMU，回执为 <b>UNSUPPORTED</b>；真实飞控将进入校准流程并在本面板提示旋转/翻面。</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";

const qgcApi = createQgcApi();
const calType = ref(1);
const step = ref(0);
const sending = ref(false);

async function startCal() {
  sending.value = true;
  step.value = 1;
  try {
    await qgcApi.sendCommand("calibrate", null, [calType.value]);
  } catch (e) {
    step.value = 0;
  } finally {
    sending.value = false;
  }
}
</script>

<style scoped>
.cal-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.cal-steps {
  margin-bottom: 4px;
}
.cal-type {
  display: flex;
  align-items: center;
  gap: 10px;
}
.cal-label {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.cal-select {
  width: 160px;
}
.cal-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.cal-warn {
  color: #e6a23c;
}
</style>
