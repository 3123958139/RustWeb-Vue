<!--
  Editor.vue —— 协议编辑器页（protocol_generator 模块）
  路由入口视图，承载 ProtocolEditor 组件。
-->
<template>
  <div class="pg-page">
    <ProtocolEditor />
  </div>
</template>

<script setup lang="ts">
import ProtocolEditor from "../components/ProtocolEditor.vue";
</script>