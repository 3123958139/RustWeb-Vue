<!--
  UDP 通信监控应用（ftj1c）根组件（薄封装）

  复用共享 AppShell（导航条 + 路由出口 + 认证初始化），
  实现位于 @shared/template/AppShell.vue。
-->
<script setup lang="ts">
import { AppShell } from "@shared"
</script>

<template>
  <AppShell />
</template>
