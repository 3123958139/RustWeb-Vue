<!--
  数据展示面板组件

  使用 Element Plus 的 el-table 显示解码后的 SharedData 字段。
  Props：tableData — TableRow[] 类型的数据行数组。

  Vue 3 Props 类型化：
  defineProps<{ tableData: TableRow[] }>() 提供编译时类型检查。
-->

<script lang="ts" setup>
import type { TableRow } from "@/fj200c_information/api/fj200c_information";

/** 声明组件 Props：接收表格数据数组 */
defineProps<{
  tableData: TableRow[];
}>();
</script>

<template>
  <el-card class="grid-cell">
    <template #header>
      <div class="card-header">
        <span class="card-title">基本信息</span>
      </div>
    </template>
    <!--
      el-table：Element Plus 表格组件
      - max-height="100%" 表格内容区自适应高度
      - size="small" 紧凑型表格
      - stripe 斑马纹样式
    -->
    <el-table :data="tableData" max-height="100%" size="small" stripe style="width: 100%">
      <el-table-column label="字段" min-width="100" prop="field" />
      <el-table-column label="值" min-width="160" prop="value" />
    </el-table>
  </el-card>
</template>

<style scoped>
.grid-cell {
  flex: 1;
}

.grid-cell :deep(.el-card__body) {
  height: calc(100% - 60px);
  overflow-y: auto;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
}
</style>
