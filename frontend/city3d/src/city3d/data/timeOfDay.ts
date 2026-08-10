/**
 * 昼夜时段配置
 *
 * 4 档时段：黎明 / 白天 / 黄昏 / 夜晚。
 * 场景引擎在切换时段时对全部参数做平滑插值过渡。
 */

export type TimeOfDayKey = "dawn" | "day" | "dusk" | "night";

export interface TimeOfDayConfig {
  key: TimeOfDayKey;
  label: string;
  /** 天空穹顶顶部颜色 */
  skyTop: string;
  /** 天空穹顶底部颜色 */
  skyBottom: string;
  /** 场景雾颜色 */
  fogColor: string;
  /** 雾近处距离 */
  fogNear: number;
  /** 雾远处距离 */
  fogFar: number;
  /** 环境光强度 */
  ambient: number;
  /** 平行光强度 */
  sun: number;
  /** 平行光颜色 */
  lightColor: string;
  /** 建筑窗户自发光强度 */
  windowEmissive: number;
  /** Bloom 辉光强度 */
  bloomStrength: number;
  /** 星空可见度（0-1） */
  starOpacity: number;
  /** 车流粒子可见度（0-1） */
  trafficOpacity: number;
  /** 太阳可见度（0-1） */
  sunOpacity: number;
  /** 月亮可见度（0-1） */
  moonOpacity: number;
  /** 太阳高度（-0.4 ~ 1.0，决定太阳方向） */
  sunHeight: number;
  /** 太阳颜色 */
  sunColor: string;
  /** 月亮颜色 */
  moonColor: string;
}

export const TIME_OF_DAY: Record<TimeOfDayKey, TimeOfDayConfig> = {
  dawn: {
    key: "dawn",
    label: "黎明",
    skyTop: "#2b3f6e",
    skyBottom: "#a8c8e8",
    fogColor: "#35486e",
    fogNear: 380,
    fogFar: 1250,
    ambient: 0.62,
    sun: 1.15,
    lightColor: "#ffd9a0",
    windowEmissive: 1.35,
    bloomStrength: 0.9,
    starOpacity: 0.25,
    trafficOpacity: 0.5,
    sunOpacity: 0.9,
    moonOpacity: 0.15,
    sunHeight: 0.12,
    sunColor: "#ffd9a0",
    moonColor: "#cfe4ff",
  },
  day: {
    key: "day",
    label: "白天",
    skyTop: "#0d47a1",
    skyBottom: "#9fd3ff",
    fogColor: "#bcd8f0",
    fogNear: 450,
    fogFar: 1500,
    ambient: 0.95,
    sun: 2.3,
    lightColor: "#ffffff",
    windowEmissive: 0.35,
    bloomStrength: 0.3,
    starOpacity: 0,
    trafficOpacity: 0.12,
    sunOpacity: 1,
    moonOpacity: 0,
    sunHeight: 0.78,
    sunColor: "#fff3e0",
    moonColor: "#dbe8ff",
  },
  dusk: {
    key: "dusk",
    label: "黄昏",
    skyTop: "#311b4d",
    skyBottom: "#ff8f5a",
    fogColor: "#5a3a5e",
    fogNear: 380,
    fogFar: 1300,
    ambient: 0.5,
    sun: 1.4,
    lightColor: "#ffb26b",
    windowEmissive: 1.9,
    bloomStrength: 1.05,
    starOpacity: 0.55,
    trafficOpacity: 0.8,
    sunOpacity: 1,
    moonOpacity: 0.3,
    sunHeight: 0.22,
    sunColor: "#ffb26b",
    moonColor: "#cfe0ff",
  },
  night: {
    key: "night",
    label: "夜晚",
    skyTop: "#050b1f",
    skyBottom: "#12294e",
    fogColor: "#0a1628",
    fogNear: 320,
    fogFar: 1150,
    ambient: 0.32,
    sun: 0.62,
    lightColor: "#6f8fc8",
    windowEmissive: 3.4,
    bloomStrength: 1.55,
    starOpacity: 1,
    trafficOpacity: 1,
    sunOpacity: 0,
    moonOpacity: 1,
    sunHeight: -0.35,
    sunColor: "#ffd9a0",
    moonColor: "#dcebff",
  },
};

export const TIME_OF_DAY_KEYS: TimeOfDayKey[] = ["dawn", "day", "dusk", "night"];
