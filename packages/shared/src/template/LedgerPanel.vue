<!--
  设备台账面板（fw100 / fw150 共享）

  从 fw100/fw150 各自逐字重复的 Panel.vue 收敛而来：
  - 角色差异仅前缀文案（roleKey / permissionKey）与 API facade（api prop）
  - 类型用结构性鸭子类型（LedgerRow），不依赖具体应用的 generated 类型
-->
<template>
  <div class="ledger-root">
    <div class="ledger-page">
      <!-- 工具栏 -->
      <div class="ledger-toolbar">
        <span class="ledger-title">设备台账</span>
        <el-tag size="small" type="success">{{ roleKey }} 角色面板</el-tag>
        <div class="spacer"></div>
        <span class="ledger-clock">权限点：{{ permissionKey }}</span>
      </div>

      <!-- 台账列表面板 -->
      <div class="ledger-panel">
        <div class="ledger-panel-header">台账列表（GET /api/{{ roleKey }}/items）</div>
        <div class="ledger-panel-body">
          <el-table v-loading="loading" :data="items" border size="small" stripe>
            <el-table-column label="编号" prop="id" width="120"/>
            <el-table-column label="名称" prop="name"/>
            <el-table-column label="类别" prop="category" width="140"/>
            <el-table-column label="状态" prop="status" width="120">
              <template #default="{ row }">
                <!-- 动态标签颜色：在线显示绿色，其他显示灰色 -->
                <el-tag :type="row.status === '在线' ? 'success' : 'info'" size="small">
                  {{ row.status }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
          <!-- 错误提示（请求失败时显示） -->
          <div v-if="errorMessage" class="ledger-error">{{ errorMessage }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import {onMounted, ref} from "vue";

/** 台账条目（结构性鸭子类型，兼容 fw100 的 LedgerItem / fw150 的 Fw150LedgerItem） */
export interface LedgerRow {
  id: string;
  name: string;
  category: string;
  status: string;
}

/** 台账 API facade（各应用传入自己的 api 对象，如 fw100Api / fw150Api） */
export interface LedgerPanelApi {
  getItems: () => Promise<{data?: LedgerRow[]}>;
}

const props = defineProps<{
  /** 角色 key（"fw100" / "fw150"），用于界面文案与 API 路径显示 */
  roleKey: string;
  /** 权限点文案（如 "fw100:monitor"） */
  permissionKey: string;
  /** 台账 API 对象 */
  api: LedgerPanelApi;
}>();

/** 台账数据列表 */
const items = ref<LedgerRow[]>([]);
/** 加载状态 */
const loading = ref(false);
/** 错误信息 */
const errorMessage = ref("");

/** 组件挂载时获取台账数据 */
onMounted(async () => {
  loading.value = true;
  try {
    const response = await props.api.getItems();
    items.value = response.data ?? []; // nullish 合并：null/undefined 时使用空数组
  } catch (error: any) {
    errorMessage.value = error?.response?.data?.message || "台账加载失败";
  } finally {
    loading.value = false;
  }
});
</script>

<style scoped>
.ledger-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
}

.ledger-page {
  padding: 16px;
  max-width: 960px;
  width: 100%;
  margin: 0 auto;
}

.ledger-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.ledger-title {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.spacer {
  flex: 1;
}

.ledger-clock {
  font-size: 13px;
  color: #909399;
}

.ledger-panel {
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 6px;
}

.ledger-panel-header {
  padding: 10px 14px;
  border-bottom: 1px solid #ebeef5;
  font-weight: 600;
  font-size: 14px;
  background: #fafafa;
  border-radius: 6px 6px 0 0;
}

.ledger-panel-body {
  padding: 14px;
}

.ledger-error {
  margin-top: 12px;
  color: #f56c6c;
  font-size: 13px;
}
</style>
