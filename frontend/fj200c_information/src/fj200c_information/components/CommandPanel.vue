<!--
  命令面板容器组件

  包含多个 CommandRow（7 个命令通道）和一个 ASCII 转十六进制工具。
  Props 通过 defineProps 类型化声明。
-->

<script lang="ts" setup>
import CommandRow from "./CommandRow.vue";
import {ref, Ref} from "vue";
import {asciiToLittleEndianBytes} from "@/fj200c_information/utils/ascii.ts";
import {bytesToHex} from "@/fj200c_information/utils/hex.ts";

/** ASCII 转换工具的输入字符串 */
const input = ref("");
/** ASCII 转换工具的输出结果 */
const output = ref("");

/**
 * 执行 ASCII 转小端十六进制
 * asciiToLittleEndianBytes 返回 Uint8Array 或错误字符串
 */
function onConvert() {
  const result = asciiToLittleEndianBytes(input.value);
  output.value = result instanceof Uint8Array ? bytesToHex(result) : result;
}

/** 命令通道类型（与 CommandRow 的 Props 一致） */
interface Channel {
  cmdType: Ref<string>;
  cmdData: Ref<string>;
  cmdOptions: string[];
  sendCommand: () => Promise<void>;
}

/** 组件 Props */
defineProps<{
  channels: Channel[];
  labels: string[];
  disabledTypes: boolean[];
  disabledDatas: boolean[];
}>();
</script>

<template>
  <el-card class="grid-cell">
    <template #header>
      <div class="card-header">
        <span class="card-title">命令通道</span>
      </div>
    </template>
    <!-- 7 个命令通道（动态渲染） -->
    <div class="right-card-body">
      <CommandRow
        v-for="(ch, i) in channels"
        :key="i"
        :channel="ch"
        :disabled-data="disabledDatas[i]"
        :disabled-type="disabledTypes[i]"
        :label="labels[i]"
      />
    </div>
    <el-divider/>
    <!-- ASCII 转十六进制工具 -->
    <div class="fj200c_information-panel">
      <div class="fj200c_information-panel-header">
        ASCII（大端，不超过8个字符）转Byte（小端，不够8个字符时低位补空格）
      </div>
      <div class="cmd-row">
          <el-input
              v-model="input"
              placeholder="不超过8个字符"
              size="small"
              style="width: 130px"
          />
            <el-input
                :model-value="output"
                placeholder="小端十六进制数据"
                readonly
                size="small"
                style="width: 180px"
            />
          <el-button size="small" type="primary" @click="onConvert">确定</el-button>
      </div>
    </div>

  </el-card>
</template>

<style scoped>
.cmd-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.grid-cell {
  flex: 1;
}

.grid-cell :deep(.el-card__body) {
  height: calc(100% - 60px);
  overflow-y: auto;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
}

.right-card-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  height: 60%;
}
</style>
