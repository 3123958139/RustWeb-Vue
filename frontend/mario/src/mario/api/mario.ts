/**
 * 超级马里奥成绩 API（mario 角色面板）
 *
 * 后端只负责成绩持久化与排行榜（`src/mario/`），游戏逻辑运行于
 * `frontend/mario/src/mario/views/GameView.vue`（Canvas）。
 * 此处直接使用本应用装配的 Axios 实例，返回体为统一 `ApiResponse<T>` 包装。
 */
import { api } from "@/api";

/** 后端统一响应包装（与 `src/common/models.rs` 的 ApiResponse 对应） */
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

/** 提交成绩请求体（与 `src/mario/models.rs::SubmitScoreRequest` 对应） */
export interface SubmitMarioScore {
  score: number;
  level: number;
  coins: number;
  time_ms: number;
}

/** 榜单条目（与 `src/mario/models.rs::MarioScore` 对应） */
export interface MarioScore {
  ranking: number;
  username: string;
  score: number;
  level: number;
  coins: number;
  time_ms: number;
  created_at: string;
}

/** 高分榜（与 `src/mario/models.rs::ScoreList` 对应） */
export interface ScoreList {
  items: MarioScore[];
  total: number;
}

/** 全局统计（与 `src/mario/models.rs::MarioStats` 对应） */
export interface MarioStats {
  total_games: number;
  total_coins: number;
  top_score: number;
  top_username: string | null;
  players: number;
}

/** 创建超级马里奥成绩 API 对象 */
export function createMarioApi() {
  return {
    /** 高分榜（按分数倒序） */
    async getScores(limit = 10): Promise<ScoreList> {
      const res = await api.get<ApiResponse<ScoreList>>("/mario/scores", { params: { limit } });
      return res.data.data;
    },

    /** 提交一局游戏成绩 */
    async submitScore(data: SubmitMarioScore): Promise<MarioScore> {
      const res = await api.post<ApiResponse<MarioScore>>("/mario/scores", data);
      return res.data.data;
    },

    /** 全局统计 */
    async getStats(): Promise<MarioStats> {
      const res = await api.get<ApiResponse<MarioStats>>("/mario/stats");
      return res.data.data;
    },
  };
}

/** 超级马里奥成绩 API 类型 */
export type MarioApi = ReturnType<typeof createMarioApi>;