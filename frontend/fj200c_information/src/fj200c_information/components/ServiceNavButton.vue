<!--
  导航栏服务开关按钮

  替代源 App.vue 原生菜单的"创建服务"项。
  通过 useService 组合式函数切换发动机监控服务的启动/停止。
  所有 fj200c_information 页面通过 AppNavbar 的 #actions 插槽挂载。
-->

<script lang="ts" setup>
import {ElMessage} from "element-plus";
import {useService} from "@/fj200c_information/composables/useService";

/**
 * 使用服务控制组合式函数
 * 解构获取：serviceRunning（运行状态）、starting/stopping（操作状态）、toggleService（切换方法）
 */
const {serviceRunning, starting, stopping, toggleService} = useService();

/** 切换服务状态（启动/停止） */
const onToggle = async () => {
  const result = await toggleService();
  if (!result.success) {
    ElMessage.error(result.message || "操作失败");
  }
};
</script>

<template>
  <!-- loading 状态：starting 或 stopping 时按钮显示加载动画 -->
  <el-button
      :loading="starting || stopping"
      class="nav-item"
      size="large"
      type="primary"
      @click="onToggle"
  >
    {{ serviceRunning ? "停止服务" : "创建服务" }}
  </el-button>
</template>

<style scoped>
.nav-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  text-decoration: none;
  background: #409eff;
  color: #ffffff;
  font-weight: 500;
  font-size: 14px;
  font-family: inherit;
  transition: all 0.3s ease;
  cursor: pointer;
  white-space: nowrap;
  height: 37px;
}

.nav-item:hover {
  background-color: #f0f9ff;
  color: #409eff;
}

.nav-item.active {
  background-color: white;
  color: white;
}
</style>
