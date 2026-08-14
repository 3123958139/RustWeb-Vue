<!--
  Csv.vue —— 默认参数表页（protocol_generator 模块）
  路由入口视图，承载 CsvEditor 组件。
-->
<template>
  <div class="pg-page">
    <CsvEditor />
  </div>
</template>

<script setup lang="ts">
import CsvEditor from "../components/CsvEditor.vue";
</script>