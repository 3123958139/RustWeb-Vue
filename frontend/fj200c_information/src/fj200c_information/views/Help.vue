<!--
  帮助页

  由导航栏"帮助"进入，替代原 App.vue 的 HelpDialog 弹框。
  展示命令通道使用说明。
-->
<template>
  <div class="fj200c_information-root">
    <div class="fj200c_information-page">
      <div class="fj200c_information-toolbar">
        <span class="toolbar-title">帮助</span>
        <div class="spacer"></div>
        <ServiceNavButton />
        <el-button size="small" @click="router.back()">返回</el-button>
      </div>

      <div class="fj200c_information-panel">
        <div class="fj200c_information-panel-header">使用说明</div>
        <div class="fj200c_information-panel-body">
          <div class="help-content">
            命令通道中：【命令类型】产品名称、发动机产品代号、电控器产品代号使用ASCII表示，
            然后使用工具将ASCII转为16进制数填入小端十六进制数据，
            其他的【命令类型】使用16进制数表示，直接填入小端十六进制数据
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import ServiceNavButton from "@/fj200c_information/components/ServiceNavButton.vue";

/** 路由实例，用于返回按钮的编程式导航 */
const router = useRouter();
</script>

<style scoped>
@import "@/fj200c_information/fj200c_information.css";

.help-content {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;   /* 保留换行和空格 */
  word-break: break-all;
}
</style>
