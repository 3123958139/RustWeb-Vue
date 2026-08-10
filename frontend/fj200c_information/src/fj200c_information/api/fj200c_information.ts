/**
 * 发动机监控模块 API（fj200c_information 角色面板）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容，WebSocket 保持手写。
 */
import { getFj200cInformation } from "@shared/api/generated";
import { buildWebSocketUrl as sharedBuildWebSocketUrl } from "@shared";

/** 类型 re-export（与视图 import 路径兼容） */
export type {
  ServiceStatus,
  SentResult,
  SavedResult,
  ConfigContent,
  CsvFileList,
  CsvFileContent,
} from "@shared/api/generated";

/** 表格行数据（字段名 + 值） */
export interface TableRow {
  field: string;
  value: string;
}

/** 帧数据事件（解码后的完整帧，包含字段数组） */
export interface Fj200cInformationFrameEvent {
  type: "frame";
  connection_index: number;
  hex: string;
  frame_type: string;
  fields: string[];
}

/** 原始载荷事件（未经解码的原始十六进制数据） */
export interface Fj200cInformationPayloadEvent {
  type: "payload";
  connection_index: number;
  hex: string;
}

/** 表格数据事件（解码后的键值对数据） */
export interface Fj200cInformationTableDataEvent {
  type: "table_data";
  connection_index: number;
  rows: TableRow[];
}

/**
 * 联合类型：三种事件类型的联合
 * 使用 TypeScript Discriminated Unions（可辨识联合），通过 type 字段区分
 */
export type Fj200cInformationEvent = Fj200cInformationFrameEvent | Fj200cInformationPayloadEvent | Fj200cInformationTableDataEvent;

/**
 * 创建发动机监控 API 对象
 *
 * @returns 包含所有发动机监控接口的 API 对象
 */
export function createFj200cInformationApi() {
  const api = getFj200cInformation();
  return {
    /** 启动发动机监控服务 */
    async startService() {
      return api.fj200cInformationStartService();
    },

    /** 停止发动机监控服务 */
    async stopService() {
      return api.fj200cInformationStopService();
    },

    /** 查询服务运行状态 */
    async getServiceStatus() {
      return api.fj200cInformationGetServiceStatus();
    },

    /**
     * 发送命令帧到设备
     * @param hex - 十六进制字符串格式的命令帧
     */
    async sendCommand(hex: string) {
      return api.fj200cInformationSendCommand({ hex });
    },

    /** 读取 config-fj200c_information.ini 配置文件内容 */
    async getConfig() {
      return api.fj200cInformationGetConfig();
    },

    /** 保存配置文件内容 */
    async saveConfig(content: string) {
      return api.fj200cInformationSaveConfig({ content });
    },

    /** 获取 CSV 文件列表 */
    async listCsvFiles() {
      return api.fj200cInformationListCsvFiles();
    },

    /**
     * 获取指定 CSV 文件内容
     * @param name - 文件名（会自动 URL 编码）
     */
    async getCsvFile(name: string) {
      return api.fj200cInformationGetCsvFile(encodeURIComponent(name));
    },

    /**
     * 构建 WebSocket 地址（复用 shared 公共实现）
     *
     * 开发环境由 Vite 代理（/api 含 ws），生产环境同源。
     * 浏览器 WebSocket API 不支持自定义 header，
     * 因此 JWT token 通过 URL query 参数传递。
     */
    buildWebSocketUrl(): string {
      return sharedBuildWebSocketUrl("/api/fj200c_information/ws");
    },
  };
}

/** 发动机监控 API 类型 */
export type Fj200cInformationApi = ReturnType<typeof createFj200cInformationApi>;
