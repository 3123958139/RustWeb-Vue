<!--
  设备台账应用（fw150）根组件（薄封装）

  复用共享 AppShell（导航条 + 路由出口 + 认证初始化），
  实现位于 @shared/template/AppShell.vue。
-->
<script lang="ts" setup>
import { AppShell } from "@shared"
</script>

<template>
  <AppShell />
</template>
