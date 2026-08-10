<!--
  管理后台登录页
  复用共享 LoginPage 模板，样式与 AppNavbar 统一
-->
<template>
  <LoginPage
    title="管理后台"
    subtitle="欢迎回来"
    app-kind="admin"
    footer-text="用户由管理员创建并分配角色"
  />
</template>

<script setup lang="ts">
import { LoginPage } from "@shared";
</script>