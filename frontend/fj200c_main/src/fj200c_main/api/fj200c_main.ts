/**
 * fj200c_main 模块 API（发动机测控主面板）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容，WebSocket 保持手写。
 */
import { getFj200cMain } from "@shared/api/generated";
import { buildWebSocketUrl as sharedBuildWebSocketUrl } from "@shared";
import type { ChannelData } from "@shared/api/generated";

/** 类型 re-export（与视图 import 路径兼容） */
export type {
  ServiceStatus,
  SentResult,
  SavedResult,
  ConfigContent,
  CsvFileList,
  CsvFileContent,
  ExperimentInfo,
  ReportOutput,
  RecordingState,
  SimulationState,
  ThemeState,
} from "@shared/api/generated";

/** 帧数据事件（解码后的完整帧，含通道索引/十六进制/字段） */
export interface PortDataEvent {
  type: "port_data";
  connection_index: number;
  hex: string;
  fields: ChannelData;
}

/** 模拟运行状态事件 */
export interface SimulationStateEvent {
  type: "simulation_state";
  simulating: boolean;
}

/** 主题状态事件 */
export interface ThemeStateEvent {
  type: "theme_state";
  isDark: boolean;
}

/** CSV 录制状态事件 */
export interface CsvRecordingStateEvent {
  type: "csv_recording_state";
  recording: boolean;
}

/** 联合类型：WS 事件按 type 字段区分 */
export type Fj200cMainWsEvent =
  | PortDataEvent
  | SimulationStateEvent
  | ThemeStateEvent
  | CsvRecordingStateEvent;

/**
 * 创建 fj200c_main API 对象
 *
 * @returns 包含 15 个 HTTP 接口 + buildWebSocketUrl 的 API 对象
 */
export function createFj200cMainApi() {
  const api = getFj200cMain();
  return {
    /** 启动测控服务 */
    async startService() {
      return api.fj200cMainStartService();
    },

    /** 停止测控服务 */
    async stopService() {
      return api.fj200cMainStopService();
    },

    /** 查询服务运行状态 */
    async getServiceStatus() {
      return api.fj200cMainGetServiceStatus();
    },

    /**
     * 发送 ECU 指令（十六进制帧）
     * @param hex - 十六进制字符串格式的命令帧
     */
    async sendCommand(hex: string) {
      return api.fj200cMainSendCommand({ hex });
    },

    /** 读取 config-fj200c_main.ini 配置文件内容 */
    async getConfig() {
      return api.fj200cMainGetConfig();
    },

    /** 保存配置文件内容 */
    async saveConfig(content: string) {
      return api.fj200cMainSaveConfig({ content });
    },

    /** 获取 CSV 文件列表 */
    async listCsvFiles() {
      return api.fj200cMainListCsvFiles();
    },

    /**
     * 获取指定 CSV 文件内容
     * @param name - 文件名（会自动 URL 编码）
     */
    async getCsvFile(name: string) {
      return api.fj200cMainGetCsvFile(encodeURIComponent(name));
    },

    /** 获取试验信息 */
    async getExperiment() {
      return api.fj200cMainGetExperiment();
    },

    /** 保存试验信息 */
    async saveExperiment(experimentInfo: import("@shared/api/generated").ExperimentInfo) {
      return api.fj200cMainSaveExperiment(experimentInfo);
    },

    /**
     * 生成试验报表
     * @param fileName - CSV 文件名（用于推导试验信息文件名）
     * @param content - CSV 文件内容
     * @param statePoints - 状态点（逗号分隔的 RPM 值）
     */
    async generateReport(fileName: string, content: string, statePoints: string) {
      return api.fj200cMainGenerateReport({ fileName, content, statePoints });
    },

    /** 获取用户操作说明（help_doc.md） */
    async getHelp() {
      return api.fj200cMainGetHelp();
    },

    /** 切换 CSV 数据录制状态 */
    async toggleRecording() {
      return api.fj200cMainToggleRecording();
    },

    /** 切换模拟运行状态 */
    async toggleSimulation() {
      return api.fj200cMainToggleSimulation();
    },

    /**
     * 设置主题（深色/浅色）
     * @param isDark - 是否深色主题
     */
    async setTheme(isDark: boolean) {
      return api.fj200cMainSetTheme({ isDark });
    },

    /**
     * 构建 WebSocket 地址（复用 shared 公共实现）
     *
     * 开发环境由 Vite 代理（/api 含 ws），生产环境同源。
     * 浏览器 WebSocket API 不支持自定义 header，
     * 因此 JWT token 通过 URL query 参数传递。
     */
    buildWebSocketUrl(): string {
      return sharedBuildWebSocketUrl("/api/fj200c_main/ws");
    },
  };
}

/** fj200c_main API 类型 */
export type Fj200cMainApi = ReturnType<typeof createFj200cMainApi>;
