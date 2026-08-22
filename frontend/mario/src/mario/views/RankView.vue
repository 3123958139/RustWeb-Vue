<!--
  高分榜 —— 超级马里奥复刻全局成绩展示
  数据来自后端 `/api/mario/scores` 与 `/api/mario/stats`。
-->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { marioApi } from "@/api";
import type { MarioScore, MarioStats } from "@/mario/api/mario";

const loading = ref(true);
const error = ref("");
const scores = ref<MarioScore[]>([]);
const stats = ref<MarioStats>({
  total_games: 0,
  total_coins: 0,
  top_score: 0,
  top_username: null,
  players: 0,
});

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [s, st] = await Promise.all([marioApi.getScores(20), marioApi.getStats()]);
    scores.value = s.items;
    stats.value = st;
  } catch (e) {
    error.value = e instanceof Error ? e.message : "加载排行榜失败";
  } finally {
    loading.value = false;
  }
}

function formatTime(ms: number): string {
  if (ms <= 0) return "--";
  return `${(ms / 1000).toFixed(1)}s`;
}

onMounted(load);
</script>

<template>
  <div class="rank-page">
    <div class="rank-header">
      <h2>🏆 马里奥高分榜</h2>
      <el-button v-loading="loading" link type="primary" @click="load">刷新</el-button>
    </div>

    <el-alert v-if="error" :title="error" type="error" show-icon :closable="false" />

    <div class="stats-row">
      <div class="stat-card">
        <div class="stat-num">{{ stats.total_games }}</div>
        <div class="stat-label">总挑战次数</div>
      </div>
      <div class="stat-card">
        <div class="stat-num">{{ stats.top_score }}</div>
        <div class="stat-label">最高分 · {{ stats.top_username ?? "—" }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-num">{{ stats.total_coins }}</div>
        <div class="stat-label">累计金币</div>
      </div>
      <div class="stat-card">
        <div class="stat-num">{{ stats.players }}</div>
        <div class="stat-label">参与玩家</div>
      </div>
    </div>

    <el-table v-loading="loading" :data="scores" class="rank-table" stripe>
      <el-table-column label="排名" width="80">
        <template #default="{ row }">
          <span class="rank-badge" :class="row.ranking <= 3 ? 'top' : ''">{{ row.ranking }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="username" label="玩家" min-width="120" />
      <el-table-column prop="score" label="得分" width="110" sortable />
      <el-table-column prop="level" label="关卡" width="80" />
      <el-table-column prop="coins" label="金币" width="80" />
      <el-table-column label="通关耗时" width="110">
        <template #default="{ row }">{{ formatTime(row.time_ms) }}</template>
      </el-table-column>
      <el-table-column prop="created_at" label="时间" min-width="140" />
    </el-table>
  </div>
</template>

<style scoped>
.rank-page {
  flex: 1;
  padding: 24px;
  max-width: 860px;
  width: 100%;
  margin: 0 auto;
  overflow-y: auto;
}
.rank-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}
.rank-header h2 {
  margin: 0;
  color: #ffcc00;
}
.stats-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}
.stat-card {
  background: rgba(18, 25, 50, 0.85);
  border: 1px solid rgba(255, 204, 0, 0.2);
  border-radius: 10px;
  padding: 14px;
  text-align: center;
}
.stat-num {
  font-size: 26px;
  font-weight: 700;
  color: #ffcc00;
}
.stat-label {
  font-size: 12px;
  color: #aaa;
  margin-top: 4px;
}
.rank-table {
  border-radius: 10px;
  overflow: hidden;
}
.rank-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: #2a3a6a;
  color: #ccc;
  font-weight: 700;
}
.rank-badge.top {
  background: #ffcc00;
  color: #0a0e1a;
}
</style>