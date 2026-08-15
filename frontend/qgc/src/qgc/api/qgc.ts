/**
 * qgc 飞控地面站模块 API（MAVLink 连接监控 + 任务规划）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容，WebSocket 事件类型保持手写。
 */
import {getQgc} from "@shared/api/generated";
import type {QgcMissionItem, QgcTelemetry} from "@shared/api/generated";
import {buildWebSocketUrl as sharedBuildWebSocketUrl} from "@shared";

/** 类型 re-export（与视图 import 路径兼容） */
export type {
    QgcTelemetry,
    QgcMission,
    QgcMissionItem,
} from "@shared/api/generated";

/**
 * 任务进度事件载荷
 *
 * 与后端 MissionProgress（src/qgc/mod.rs）字段一致。
 * state: idle / uploading / downloading / clearing。
 */
export interface QgcMissionProgressPayload {
    state: string;
    /** 总条目数（含首页） */
    total: number;
    /** 已处理条目数 */
    received: number;
    /** 结果描述：ok / timeout / 错误消息 */
    result: string;
    /** 飞控当前执行航点序号（-1 = 不在任务中） */
    current_seq: number;
}

/**
 * 命令回执事件载荷
 *
 * 与后端 CommandAckPayload（src/qgc/mod.rs）字段一致。
 */
export interface QgcCommandAckPayload {
    /** 原始命令（MAV_CMD 枚举值） */
    command: number;
    /** MAV_RESULT 结果码 */
    result: number;
    /** 结果码名称（ACCEPTED / DENIED / FAILED 等） */
    result_name: string;
}

/**
 * WebSocket 事件（内部标签序列化，`event` 字段区分类型）
 *
 * 与后端 QgcEvent（src/qgc/mod.rs）一致。
 */
export type QgcWsEvent =
    | ({ event: "telemetry" } & Partial<QgcTelemetry>)
    | ({ event: "mission_progress" } & QgcMissionProgressPayload)
    | ({ event: "command_ack" } & QgcCommandAckPayload);

/**
 * 创建飞控地面站 API 对象
 *
 * @returns 包含所有飞控地面站接口的 API 对象
 */
export function createQgcApi() {
    const api = getQgc();
    return {
        /** 启动飞控通信服务（排他，自动停止其他角色后台线程） */
        async startService() {
            return api.qgcStartService();
        },

        /** 停止飞控通信服务（异步，HTTP 立即返回） */
        async stopService() {
            return api.qgcStopService();
        },

        /** 查询服务运行状态 */
        async getServiceStatus() {
            return api.qgcGetServiceStatus();
        },

        /** 获取遥测快照 */
        async getTelemetry() {
            return api.qgcGetTelemetry();
        },

        /** 发送飞控命令（arm / disarm / takeoff / land / rtl / start / pause / resume / click_to_go / move） */
        async sendCommand(command: string, altitude?: number | null, params?: number[] | null) {
            return api.qgcSendCommand({command, altitude: altitude ?? null, params: params ?? []});
        },

        /** 切换飞行模式（ArduPilot Copter 模式名或数字 ID） */
        async setMode(mode: string) {
            return api.qgcSetMode({mode});
        },

        /** 获取任务快照（状态 + 航点列表） */
        async getMission() {
            return api.qgcGetMission();
        },

        /** 上传任务（自动补首页，序号自动重排） */
        async uploadMission(items: QgcMissionItem[]) {
            return api.qgcUploadMission({items});
        },

        /** 从飞控下载当前任务 */
        async downloadMission() {
            return api.qgcDownloadMission();
        },

        /** 清除飞控任务 */
        async clearMission() {
            return api.qgcClearMission();
        },

        /** 读取配置文件内容（config-qgc.ini） */
        async getConfig() {
            return api.qgcGetConfig();
        },

        /** 保存配置文件内容（重启服务后生效） */
        async saveConfig(content: string) {
            return api.qgcSaveConfig({content});
        },

        /** 获取帮助文档（help_doc.md） */
        async getHelp() {
            return api.qgcGetHelp();
        },

        /**
         * 构建 WebSocket 地址（复用 shared 公共实现）
         *
         * 浏览器 WebSocket API 不支持自定义 header，
         * JWT token 通过 URL query 参数传递。
         */
        buildWebSocketUrl(): string {
            return sharedBuildWebSocketUrl("/api/qgc/ws");
        },
    };
}

/** 飞控地面站 API 类型 */
export type QgcApi = ReturnType<typeof createQgcApi>;
