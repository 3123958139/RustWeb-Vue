<script lang="ts" setup>
import {onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {ElMessage} from 'element-plus'
import {ArrowLeft} from '@element-plus/icons-vue'
import {city3dApi} from '@/api'
import type {Building, CityEvent, CityOverview} from '@/city3d/api/city3d'

const router = useRouter()

const overview = ref<CityOverview | null>(null)
const buildings = ref<Building[]>([])
const districts = ref<any[]>([])
const events = ref<CityEvent[]>([])
const activeTab = ref('overview')
const loading = ref(true)

onMounted(async () => {
  try {
    const [overviewRes, buildingsRes, districtsRes, eventsRes] = await Promise.all([
      city3dApi.getOverview(),
      city3dApi.getBuildings(),
      city3dApi.getDistricts(),
      city3dApi.getEvents(),
    ])
    if (overviewRes.success) overview.value = overviewRes.data ?? null
    if (buildingsRes.success) buildings.value = buildingsRes.data?.items ?? []
    if (districtsRes.success) districts.value = districtsRes.data ?? []
    if (eventsRes.success) events.value = eventsRes.data?.items ?? []
  } catch (e) {
    ElMessage.error('加载数据失败')
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="data-panel">
    <div class="panel-header">
      <el-button class="back-btn" text @click="router.push('/city3d/main')">
        <el-icon>
          <ArrowLeft/>
        </el-icon>
        返回场景
      </el-button>
      <h1 class="panel-title">数据面板</h1>
    </div>
    <div class="panel-content">
      <el-tabs v-model="activeTab" class="data-tabs">
        <el-tab-pane label="概览" name="overview">
          <div v-loading="loading" class="tab-content">
            <div class="stats-grid">
              <el-card class="stat-card" shadow="never">
                <div class="stat-inner">
                  <span class="stat-num">{{ overview?.total_buildings ?? 0 }}</span>
                  <span class="stat-desc">建筑总数</span>
                </div>
              </el-card>
              <el-card class="stat-card" shadow="never">
                <div class="stat-inner">
                  <span class="stat-num">{{ overview?.total_districts ?? 0 }}</span>
                  <span class="stat-desc">区域总数</span>
                </div>
              </el-card>
              <el-card class="stat-card" shadow="never">
                <div class="stat-inner">
                  <span class="stat-num">{{ overview?.total_energy_kw ?? 0 }} kW</span>
                  <span class="stat-desc">总能耗</span>
                </div>
              </el-card>
              <el-card class="stat-card" shadow="never">
                <div class="stat-inner">
                  <span class="stat-num">{{ overview?.total_population ?? 0 }}</span>
                  <span class="stat-desc">总人口</span>
                </div>
              </el-card>
            </div>
          </div>
        </el-tab-pane>
        <el-tab-pane label="建筑" name="buildings">
          <div v-loading="loading" class="tab-content">
            <el-table :data="buildings" size="small" stripe style="width: 100%">
              <el-table-column label="名称" min-width="120" prop="name"/>
              <el-table-column label="所属区域" min-width="100" prop="district_name"/>
              <el-table-column label="高度(m)" prop="height" width="90"/>
              <el-table-column label="楼层" prop="floors" width="70"/>
              <el-table-column label="状态" prop="status" width="80"/>
              <el-table-column label="能耗(kW)" prop="energy_kw" width="90"/>
            </el-table>
          </div>
        </el-tab-pane>
        <el-tab-pane label="区域" name="districts">
          <div v-loading="loading" class="tab-content">
            <el-table :data="districts" size="small" stripe style="width: 100%">
              <el-table-column label="名称" min-width="120" prop="name"/>
              <el-table-column label="编码" prop="code" width="100"/>
              <el-table-column label="建筑数量" prop="building_count" width="100"/>
              <el-table-column label="描述" min-width="200" prop="description"/>
            </el-table>
          </div>
        </el-tab-pane>
        <el-tab-pane label="事件" name="events">
          <div v-loading="loading" class="tab-content">
            <el-table :data="events" size="small" stripe style="width: 100%">
              <el-table-column label="类型" prop="type" width="100"/>
              <el-table-column label="标题" min-width="200" prop="title"/>
              <el-table-column label="描述" min-width="250" prop="description"/>
              <el-table-column label="时间" prop="created_at" width="180"/>
            </el-table>
          </div>
        </el-tab-pane>
      </el-tabs>
    </div>
  </div>
</template>

<style scoped>
.el-table {
  --el-table-bg-color: transparent;
  --el-table-header-bg-color: black;
  --el-table-text-color: #a0b0d0 !important;
  --el-table-row-hover-bg-color: rgba(255, 255, 255, 0.1);
  --el-table-border: 1;
  --el-table-tr-bg-color: rgba(0, 212, 255, 0.1);
}

.data-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: #0a0e1a;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  background: rgba(18, 25, 50, 0.9);
  border-bottom: 1px solid rgba(0, 212, 255, 0.1);
}

.back-btn {
  color: #a0b0d0 !important;
  font-size: 14px;
}

.back-btn:hover {
  color: #00d4ff !important;
}

.panel-title {
  font-size: 20px;
  font-weight: 600;
  color: #00d4ff;
  letter-spacing: 2px;
  margin: 0;
}

.panel-content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

.data-tabs :deep(.el-tabs__item) {
  color: #a0b0d0;
  font-size: 15px;
  letter-spacing: 1px;
}

.data-tabs :deep(.el-tabs__item.is-active) {
  color: #00d4ff;
}

.data-tabs :deep(.el-tabs__active-bar) {
  background-color: #00d4ff;
}

.data-tabs :deep(.el-tabs__nav-wrap::after) {
  background-color: rgba(0, 212, 255, 0.1);
}

.tab-content {
  padding: 16px 0;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.stat-card {
  border-radius: 12px;
}

.stat-inner {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 0;
}

.stat-num {
  font-size: 28px;
  font-weight: 700;
  color: #00d4ff;
  font-variant-numeric: tabular-nums;
}

.stat-desc {
  font-size: 13px;
  color: #a0b0d0;
  letter-spacing: 1px;
}
</style>