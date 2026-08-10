<!--
  城市 3D 全景页顶部控制栏

  包含：品牌标题 / 昼夜时段切换 / 天气切换 / 热力模式 / 自动旋转 / 重新加载 / 用户菜单
-->
<template>
  <div class="hud-topbar">
    <div class="hud-brand">
      <div class="hud-brand-icon">
        <el-icon :size="22"><OfficeBuilding /></el-icon>
      </div>
      <div class="hud-brand-text">
        <span class="hud-title">城市数字孪生</span>
        <span class="hud-subtitle">CITY DIGITAL TWIN · 三维可视化平台</span>
      </div>
    </div>

    <div class="hud-controls">
      <!-- 昼夜时段 -->
      <div class="control-group">
        <span class="control-label">时段</span>
        <div class="segmented">
          <button
            v-for="item in TIME_OF_DAY_KEYS"
            :key="item"
            class="segment"
            :class="{ active: timeOfDay === item }"
            @click="$emit('update:timeOfDay', item)"
          >
            {{ TIME_OF_DAY[item].label }}
          </button>
        </div>
      </div>

      <!-- 天气 -->
      <div class="control-group">
        <span class="control-label">天气</span>
        <div class="segmented">
          <button
            v-for="item in weatherOptions"
            :key="item.key"
            class="segment"
            :class="{ active: weather === item.key }"
            @click="$emit('update:weather', item.key)"
          >
            {{ item.label }}
          </button>
        </div>
      </div>

      <!-- 模式开关 -->
      <div class="control-group switch-group">
        <el-switch
          :model-value="heatMode"
          size="small"
          @update:model-value="$emit('update:heatMode', $event)"
        />
        <span class="control-label">能耗热力</span>
      </div>
      <div class="control-group switch-group">
        <el-switch
          :model-value="autoRotate"
          size="small"
          @update:model-value="$emit('update:autoRotate', $event)"
        />
        <span class="control-label">自动旋转</span>
      </div>

      <div class="control-group">
        <el-tooltip content="重新加载场景数据" placement="bottom">
          <el-button size="small" :icon="Refresh" circle @click="$emit('reload')" />
        </el-tooltip>
      </div>

      <!-- 用户菜单 -->
      <el-dropdown @command="handleCommand">
        <span class="user-chip">
          <el-avatar :size="26">{{ user?.username?.charAt(0)?.toUpperCase() }}</el-avatar>
          <span class="user-name">{{ user?.username }}</span>
          <el-icon><ArrowDown /></el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item disabled>{{ getUserRoleText() }}</el-dropdown-item>
            <el-dropdown-item divided command="logout">退出登录</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ElMessage } from "element-plus";
import { useRouter } from "vue-router";
import { ArrowDown, OfficeBuilding, Refresh } from "@element-plus/icons-vue";
import { getAppAuthStore, findRole } from "@shared";
import { TIME_OF_DAY, TIME_OF_DAY_KEYS, type TimeOfDayKey } from "@/city3d/data/timeOfDay";
import type { WeatherKey } from "@/city3d/composables/useCityScene";

defineProps<{
  timeOfDay: TimeOfDayKey;
  weather: WeatherKey;
  heatMode: boolean;
  autoRotate: boolean;
}>();

defineEmits<{
  (e: "update:timeOfDay", value: TimeOfDayKey): void;
  (e: "update:weather", value: WeatherKey): void;
  (e: "update:heatMode", value: boolean): void;
  (e: "update:autoRotate", value: boolean): void;
  (e: "reload"): void;
}>();

const weatherOptions: { key: WeatherKey; label: string }[] = [
  { key: "none", label: "晴朗" },
  { key: "rain", label: "降雨" },
  { key: "snow", label: "降雪" },
  { key: "fog", label: "浓雾" },
];

const router = useRouter();
const authStore = getAppAuthStore<{ user: { username: string; role: string } | null; logout: () => void }>();
const user = authStore?.user ?? null;

const getUserRoleText = (): string => {
  const role = user?.role;
  return role ? (findRole(role)?.name ?? role) : "";
};

const handleCommand = (command: string): void => {
  if (command === "logout") {
    authStore?.logout();
    ElMessage.success("已退出登录");
    router.push("/login");
  }
};
</script>

<style scoped>
.hud-topbar {
  position: absolute;
  top: 16px;
  left: 16px;
  right: 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 18px;
  background: rgba(8, 14, 28, 0.62);
  border: 1px solid rgba(0, 212, 255, 0.22);
  border-radius: 12px;
  backdrop-filter: blur(14px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45);
  z-index: 20;
}

.hud-brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.hud-brand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: linear-gradient(135deg, rgba(0, 212, 255, 0.25), rgba(124, 77, 255, 0.25));
  border: 1px solid rgba(0, 212, 255, 0.4);
  color: #66e0ff;
}

.hud-brand-text {
  display: flex;
  flex-direction: column;
}

.hud-title {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: 2px;
  background: linear-gradient(90deg, #66e0ff, #9a8cff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.hud-subtitle {
  font-size: 10px;
  color: rgba(160, 200, 240, 0.6);
  letter-spacing: 1.5px;
}

.hud-controls {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.control-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.control-label {
  font-size: 12px;
  color: rgba(160, 200, 240, 0.75);
  white-space: nowrap;
}

.switch-group {
  gap: 5px;
}

.segmented {
  display: flex;
  gap: 2px;
  padding: 3px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(0, 212, 255, 0.15);
  border-radius: 8px;
}

.segment {
  padding: 4px 12px;
  font-size: 12px;
  color: rgba(200, 225, 250, 0.7);
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.25s ease;
  white-space: nowrap;
}

.segment:hover {
  color: #fff;
  background: rgba(0, 212, 255, 0.12);
}

.segment.active {
  color: #04121f;
  background: linear-gradient(135deg, #00d4ff, #5a8cff);
  font-weight: 600;
  box-shadow: 0 0 12px rgba(0, 212, 255, 0.45);
}

.user-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 20px;
  cursor: pointer;
  background: rgba(0, 212, 255, 0.08);
  border: 1px solid rgba(0, 212, 255, 0.2);
  color: #cfe0f5;
  transition: background 0.25s ease;
}

.user-chip:hover {
  background: rgba(0, 212, 255, 0.16);
}

.user-name {
  font-size: 13px;
  color: #cfe0f5;
}

@media (max-width: 900px) {
  .hud-subtitle {
    display: none;
  }
  .hud-controls {
    gap: 8px;
  }
  .user-name {
    display: none;
  }
}
</style>
