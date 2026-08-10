/**
 * 城市 3D 数据加载组合式函数
 *
 * 负责从后端拉取城市概览 / 区域 / 建筑 / 事件数据，
 * 事件流支持定时轮询（默认 5 秒），组件卸载时自动清理。
 */
import { onScopeDispose, ref } from "vue";
import { city3dApi } from "@/api";
import type { Building, CityEvent, CityOverview, District, DistrictSummary } from "@/city3d/api/city3d";

export function useCityData(
  pollIntervalMs = 5000,
  options: { silentEventErrors?: boolean } = {},
) {
  const overview = ref<CityOverview | null>(null);
  const districts = ref<DistrictSummary[]>([]);
  const buildings = ref<Building[]>([]);
  const events = ref<CityEvent[]>([]);
  const loading = ref(false);
  const errorMessage = ref("");

  let eventTimer: number | null = null;

  async function loadOverview() {
    try {
      const response = await city3dApi.getOverview();
      if (response.success) {
        overview.value = response.data ?? null;
      }
    } catch (error: any) {
      errorMessage.value = error?.response?.data?.message || "概览数据加载失败";
    }
  }

  async function loadDistricts() {
    try {
      const response = await city3dApi.getDistricts();
      if (response.success) {
        const raw = (response.data ?? []) as District[];
        districts.value = raw.map((d) => ({
          district: d,
          building_count: d.building_count,
          total_energy_kw: 0,
        }));
      }
    } catch (error: any) {
      errorMessage.value = error?.response?.data?.message || "区域数据加载失败";
    }
  }

  async function loadBuildings() {
    try {
      const response = await city3dApi.getBuildings();
      if (response.success) {
        buildings.value = response.data?.items ?? [];
      }
    } catch (error: any) {
      errorMessage.value = error?.response?.data?.message || "建筑数据加载失败";
    }
  }

  async function loadEvents() {
    try {
      const response = await city3dApi.getEvents();
      if (response.success) {
        events.value = response.data?.items ?? [];
      }
    } catch (error: any) {
      // CityScene 传入 silentEventErrors: true 静默轮询错误，
      // 避免瞬时故障长期挂着错误横幅（与原内联实现行为一致）
      if (!options.silentEventErrors) {
        errorMessage.value = error?.response?.data?.message || "事件数据加载失败";
      }
    }
  }

  /** 全量加载（概览 + 区域 + 建筑 + 事件） */
  async function loadAll() {
    loading.value = true;
    errorMessage.value = "";
    try {
      await Promise.all([loadOverview(), loadDistricts(), loadBuildings(), loadEvents()]);
    } finally {
      loading.value = false;
    }
  }

  /** 仅刷新 3D 场景相关数据（建筑 + 区域 + 概览），不中断事件轮询 */
  async function reloadSceneData() {
    await Promise.all([loadBuildings(), loadDistricts(), loadOverview()]);
  }

  function startEventPolling() {
    stopEventPolling();
    eventTimer = window.setInterval(loadEvents, pollIntervalMs);
  }

  function stopEventPolling() {
    if (eventTimer !== null) {
      window.clearInterval(eventTimer);
      eventTimer = null;
    }
  }

  onScopeDispose(() => {
    stopEventPolling();
  });

  return {
    overview,
    districts,
    buildings,
    events,
    loading,
    errorMessage,
    loadAll,
    loadEvents,
    reloadSceneData,
    startEventPolling,
    stopEventPolling,
  };
}
