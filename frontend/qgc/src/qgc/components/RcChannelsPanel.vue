<!--
  RC 通道条面板（qgc RcChannelsPanel）

  解析 RC_CHANNELS 各通道原始 PWM（1000~2000 微秒），以通道条展示
  遥控器输入；并展示 rssi 信号强度。对应后端 telemetry 的
  rc_channels / rc_rssi 字段（经 WS telemetry 事件实时推送）。
-->
<template>
  <div class="rc-panel" v-if="telemetry">
    <div class="rc-rssi">
      <span class="rc-rssi-label">接收机信号</span>
      <span class="rc-rssi-val" :class="rssiClass">{{ rssiText }}</span>
    </div>
    <div class="rc-bars">
      <div class="rc-bar" v-for="(ch, i) in visibleChannels" :key="i">
        <span class="rc-ch-name">CH{{ i + 1 }}</span>
        <div class="rc-track">
          <div class="rc-fill" :class="fillClass(ch)" :style="{ width: pct(ch) + '%' }" />
        </div>
        <span class="rc-ch-val">{{ ch }}</span>
      </div>
    </div>
  </div>
  <div v-else class="rc-empty">等待遥测数据…</div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { QgcTelemetry } from "@/qgc/api/qgc";

const props = defineProps<{ telemetry?: Partial<QgcTelemetry> }>();

// 展示前 8 路通道（其余通常未使用）
const visibleChannels = computed(() => (props.telemetry?.rc_channels ?? []).slice(0, 8));

// PWM 1000~2000 映射到 0~100%
function pct(v: number): number {
  const p = ((v - 1000) / 1000) * 100;
  return Math.max(0, Math.min(100, p));
}

function fillClass(v: number): string {
  const p = pct(v);
  if (p > 80 || p < 20) return "extreme";
  return "normal";
}

const rssiText = computed(() => {
  const r = props.telemetry?.rc_rssi ?? 0;
  return r === 0 ? "丢失" : String(r);
});

const rssiClass = computed(() => {
  const r = props.telemetry?.rc_rssi ?? 0;
  if (r === 0) return "bad";
  if (r < 40) return "warn";
  return "ok";
});
</script>

<style scoped>
.rc-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rc-rssi {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}
.rc-rssi-label {
  color: var(--el-text-color-secondary);
}
.rc-rssi-val.ok {
  color: #67c23a;
  font-weight: 600;
}
.rc-rssi-val.warn {
  color: #e6a23c;
  font-weight: 600;
}
.rc-rssi-val.bad {
  color: #f56c6c;
  font-weight: 600;
}
.rc-bars {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.rc-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.rc-ch-name {
  width: 36px;
  color: var(--el-text-color-secondary);
}
.rc-track {
  flex: 1;
  height: 10px;
  background: var(--el-fill-color-light);
  border-radius: 5px;
  overflow: hidden;
}
.rc-fill {
  height: 100%;
  border-radius: 5px;
  transition: width 0.1s linear;
}
.rc-fill.normal {
  background: var(--el-color-primary);
}
.rc-fill.extreme {
  background: #e6a23c;
}
.rc-ch-val {
  width: 48px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.rc-empty {
  color: var(--el-text-color-secondary);
  padding: 20px;
  text-align: center;
}
</style>
