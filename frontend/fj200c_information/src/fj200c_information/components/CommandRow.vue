<!--
  单条命令行组件

  每条命令通道的 UI：命令类型选择框 + 小端十六进制数据输入框 + 发送按钮。
  Props 通过 defineProps 类型化声明。

  Props：
  - channel: 命令通道对象（包含 cmdType / cmdData / cmdOptions / sendCommand）
  - disabledType: 是否禁用命令类型选择
  - disabledData: 是否禁用数据输入
  - label: 发送按钮显示文字
-->

<script lang="ts" setup>
import type { Ref } from "vue";

/**
 * Props 类型定义
 *
 * channel.cmdType 是 Ref<string> 类型（响应式引用），
 * 在模板中需要通过 .value 访问其值。
 * Vue 3 模板会自动解包 ref，所以模板中直接用 channel.cmdType。
 */
defineProps<{
  channel: {
    cmdType: Ref<string>;
    cmdData: Ref<string>;
    cmdOptions: string[];
    sendCommand: () => Promise<void>;
  };
  disabledType: boolean;
  disabledData: boolean;
  label: string;
}>();
</script>

<template>
  <div class="cmd-row">
    <!-- 命令类型选择框（可禁用） -->
    <el-select
      v-model="channel.cmdType.value"
      :disabled="disabledType"
      placeholder="命令类型"
      size="small"
      style="width: 130px"
    >
      <el-option v-for="opt in channel.cmdOptions" :key="opt" :label="opt" :value="opt" />
    </el-select>
    <!-- 十六进制数据输入框（可禁用） -->
    <el-input
      v-model="channel.cmdData.value"
      :disabled="disabledData"
      placeholder="小端十六进制数据"
      size="small"
      style="width: 180px"
    />
    <!-- 发送按钮 -->
    <el-button size="small" type="primary" @click="channel.sendCommand()">
      {{ label }}
    </el-button>
  </div>
</template>

<style scoped>
.cmd-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
</style>
