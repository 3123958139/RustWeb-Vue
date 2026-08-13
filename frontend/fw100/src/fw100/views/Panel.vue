<!--
  fw100 角色面板（设备台账）薄封装

  复用共享 LedgerPanel（@shared/template/LedgerPanel.vue），
  角色差异（前缀文案 / 权限点 / API facade）经 props 注入。
-->
<template>
  <LedgerPanel role-key="fw100" permission-key="fw100:monitor" :api="fw100Api" />
</template>

<script lang="ts" setup>
import { LedgerPanel } from "@shared";
import { fw100Api } from "@/api";
</script>
