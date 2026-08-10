<!--
  fw100 角色面板（设备台账）

  功能：
  1. 展示设备台账列表（编号、名称、类别、状态）
  2. 从后端 /api/fw100/items 获取数据
  3. 状态标签显示（在线/离线）
-->
<template>
  <div class="fw100-root">
    <!-- 全局导航条由 App.vue 渲染（登录页除外） -->

    <div class="fw100-page">
      <!-- 工具栏 -->
      <div class="fw100-toolbar">
        <span class="fw100-title">设备台账</span>
        <el-tag size="small" type="success">fw100 角色面板</el-tag>
        <div class="spacer"></div>
        <span class="fw100-clock">权限点：fw100:monitor</span>
      </div>

      <!-- 台账列表面板 -->
      <div class="fw100-panel">
        <div class="fw100-panel-header">台账列表（GET /api/fw100/items）</div>
        <div class="fw100-panel-body">
          <el-table v-loading="loading" :data="items" border size="small" stripe>
            <el-table-column label="编号" prop="id" width="100"/>
            <el-table-column label="名称" prop="name"/>
            <el-table-column label="类别" prop="category" width="140"/>
            <el-table-column label="状态" prop="status" width="100">
              <template #default="{ row }">
                <!-- 动态标签颜色：在线显示绿色，其他显示灰色 -->
                <el-tag :type="row.status === '在线' ? 'success' : 'info'" size="small">
                  {{ row.status }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
          <!-- 错误提示（请求失败时显示） -->
          <div v-if="errorMessage" class="fw100-error">{{ errorMessage }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import {onMounted, ref} from "vue";
import {fw100Api} from "@/api";
import type {LedgerItem} from "@/types";

/** 台账数据列表 */
const items = ref<LedgerItem[]>([]);
/** 加载状态 */
const loading = ref(false);
/** 错误信息 */
const errorMessage = ref("");

/**
 * 组件挂载时获取台账数据
 *
 * async/await 异步获取数据，try/catch/finally 处理错误和清理。
 */
onMounted(async () => {
  loading.value = true;
  try {
    const response = await fw100Api.getItems();
    items.value = response.data ?? []; // nullish 合并：null/undefined 时使用空数组
  } catch (error: any) {
    errorMessage.value = error?.response?.data?.message || "台账加载失败";
  } finally {
    loading.value = false;
  }
});
</script>

<style scoped>
.fw100-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
}

.fw100-page {
  padding: 16px;
  max-width: 960px;
  width: 100%;
  margin: 0 auto;
}

.fw100-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.fw100-title {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.spacer {
  flex: 1;
}

.fw100-clock {
  font-size: 13px;
  color: #909399;
}

.fw100-panel {
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 6px;
}

.fw100-panel-header {
  padding: 10px 14px;
  border-bottom: 1px solid #ebeef5;
  font-weight: 600;
  font-size: 14px;
  background: #fafafa;
  border-radius: 6px 6px 0 0;
}

.fw100-panel-body {
  padding: 14px;
}

.fw100-error {
  margin-top: 12px;
  color: #f56c6c;
  font-size: 13px;
}
</style>
