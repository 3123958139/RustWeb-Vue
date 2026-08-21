<!--
  手柄控制面板（qgc GamepadPanel）

  监听浏览器 Gamepad API，将左摇杆映射为机体水平速度（vx/vy）、
  左右扳机映射为垂直速度（vz），经「move」命令（SET_POSITION_TARGET_LOCAL_NED）
  下发，等价于键盘 WASD 控制。需先连接手柄并解锁（armed）后生效。
-->
<template>
  <div class="gamepad-panel">
    <div class="gp-tip" v-if="!supported">当前浏览器不支持 Gamepad API</div>
    <template v-else>
      <div class="gp-row">
        <span class="gp-label">状态</span>
        <span :class="connected ? 'ok' : 'dim'">{{ connected ? "已连接：" + padName : "未检测到手柄" }}</span>
      </div>
      <el-switch v-model="enabled" active-text="启用控制" inactive-text="停用" :disabled="!connected" />
      <div class="gp-axes" v-if="enabled">
        <div class="gp-axis"><span>前后(vx)</span><span>{{ axes[1].toFixed(2) }}</span></div>
        <div class="gp-axis"><span>左右(vy)</span><span>{{ axes[0].toFixed(2) }}</span></div>
        <div class="gp-axis"><span>升降(vz)</span><span>{{ axes[2].toFixed(2) }}</span></div>
      </div>
      <div class="gp-hint">摇杆推动即发送速度指令（解锁后生效；松开自动悬停）</div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { createQgcApi } from "@/qgc/api/qgc";

const qgcApi = createQgcApi();

const supported = typeof navigator !== "undefined" && "getGamepads" in navigator;
const enabled = ref(false);
const connected = ref(false);
const padName = ref("");
const axes = ref<number[]>([0, 0, 0]); // [vx_source?, vy_source?, vz_source?] 仅用于显示

let timer: number | undefined;

const SPEED = 3.0; // 最大速度 m/s
const DEADZONE = 0.12;

function applyDeadzone(v: number): number {
  return Math.abs(v) < DEADZONE ? 0 : v;
}

function poll() {
  if (!supported) return;
  const pads = navigator.getGamepads();
  let gp: Gamepad | null = null;
  for (const p of pads) {
    if (p) {
      gp = p;
      break;
    }
  }
  if (!gp) {
    connected.value = false;
    padName.value = "";
    return;
  }
  connected.value = true;
  padName.value = gp.id;
  if (!enabled.value) return;

  const ax = gp.axes;
  // 左摇杆：axes[0]=X（右正），axes[1]=Y（下正）；右扳机/左扳机：buttons[7]/[6]
  const vy = applyDeadzone(ax[0] ?? 0) * SPEED; // 机体右为正
  const vx = -applyDeadzone(ax[1] ?? 0) * SPEED; // 机体前为正（上推）
  const rt = gp.buttons[7]?.value ?? 0;
  const lt = gp.buttons[6]?.value ?? 0;
  const vz = applyDeadzone(rt - lt) * SPEED; // 升为正

  axes.value = [applyDeadzone(ax[0] ?? 0), applyDeadzone(ax[1] ?? 0), applyDeadzone(rt - lt)];

  // 仅在有效输入时下发（避免静默悬停刷屏）
  if (Math.abs(vx) > 0.01 || Math.abs(vy) > 0.01 || Math.abs(vz) > 0.01) {
    qgcApi.sendCommand("move", null, [vx, vy, vz]);
  }
}

watch(enabled, (on) => {
  if (on) {
    timer = window.setInterval(poll, 100);
  } else if (timer !== undefined) {
    clearInterval(timer);
    timer = undefined;
  }
});

onMounted(() => {
  if (supported) timer = window.setInterval(poll, 200);
});
onUnmounted(() => {
  if (timer !== undefined) clearInterval(timer);
});
</script>

<style scoped>
.gamepad-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.gp-row {
  display: flex;
  gap: 10px;
  align-items: center;
  font-size: 13px;
}
.gp-label {
  color: var(--el-text-color-secondary);
}
.ok {
  color: #67c23a;
}
.dim {
  color: var(--el-text-color-secondary);
}
.gp-axes {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.gp-axis {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}
.gp-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
