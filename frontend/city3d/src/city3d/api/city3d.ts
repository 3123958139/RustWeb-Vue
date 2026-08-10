/**
 * 城市 3D 模块 API（city3d 角色面板）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容，WebSocket 保持手写。
 */
import { getCity3d } from "@shared/api/generated";

/** 本地使用的类型（文件内引用需要 import 绑定） */
import type {
  District,
  CreateBuildingRequest,
  UpdateBuildingRequest,
  CreateDistrictRequest,
  UpdateDistrictRequest,
  CreateEventRequest,
} from "@shared/api/generated";

/** 类型 re-export（与视图 import 路径兼容，不创建本地绑定） */
export type {
  Building,
  CityEvent,
  District,
  Overview as CityOverview,
  CreateResult,
  BuildingPage,
  EventPage,
  RecentEvent,
  CreateBuildingRequest,
  UpdateBuildingRequest,
  CreateDistrictRequest,
  UpdateDistrictRequest,
  CreateEventRequest,
} from "@shared/api/generated";

/** 区域摘要（前端本地组合类型：区域 + 建筑数量 + 能量统计） */
export interface DistrictSummary {
  district: District;
  building_count: number;
  total_energy_kw: number;
}

/**
 * 创建城市 3D API 对象
 *
 * @returns 包含所有城市 3D 接口的 API 对象
 */
export function createCity3dApi() {
  const api = getCity3d();
  return {
    // Buildings
    async getBuildings(page = 1, pageSize = 200) {
      return api.city3dListBuildings({ page, page_size: pageSize });
    },
    async createBuilding(data: CreateBuildingRequest) {
      return api.city3dCreateBuilding(data);
    },
    async updateBuilding(id: string, data: UpdateBuildingRequest) {
      return api.city3dUpdateBuilding(id, data);
    },
    async deleteBuilding(id: string) {
      return api.city3dDeleteBuilding(id);
    },
    // Districts
    async getDistricts() {
      return api.city3dListDistricts();
    },
    async createDistrict(data: CreateDistrictRequest) {
      return api.city3dCreateDistrict(data);
    },
    async updateDistrict(id: string, data: UpdateDistrictRequest) {
      return api.city3dUpdateDistrict(id, data);
    },
    async deleteDistrict(id: string) {
      return api.city3dDeleteDistrict(id);
    },
    // Events
    async getEvents(page = 1, pageSize = 20) {
      return api.city3dListEvents({ page, page_size: pageSize });
    },
    async createEvent(data: CreateEventRequest) {
      return api.city3dCreateEvent(data);
    },
    async deleteEvent(id: string) {
      return api.city3dDeleteEvent(id);
    },
    // Overview
    async getOverview() {
      return api.city3dGetOverview();
    },
  };
}

/** 城市 3D API 类型 */
export type City3dApi = ReturnType<typeof createCity3dApi>;
