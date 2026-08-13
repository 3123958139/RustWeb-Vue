/**
 * ftj1c 通信监控模块 API（UDP 组播监控面板）
 *
 * 基于 orval 生成的客户端（@shared/api/generated），
 * 方法签名与视图调用点保持兼容，WebSocket 保持手写。
 */
import {getFtj1c} from "@shared/api/generated";
import {buildWebSocketUrl as sharedBuildWebSocketUrl} from "@shared";

/** 类型 re-export（与视图 import 路径兼容） */
export type {
    ServiceStatus,
    SavedResult,
    ConfigContent,
    IpConfig,
} from "@shared/api/generated";

/**
 * UDP 数据事件载荷
 *
 * 与后端 Ftj1cEvent::UdpData 字段一致。
 * 每个 UDP 数据包包含源/目的 IP、端口、原始数据和提取数据。
 */
export interface UdpDataPayload {
    /** 连接序号 0..7，前端映射为第 connection_index*2 与 *2+1 张卡片 */
    connection_index: number;
    /** 槽位序号 0..3 */
    slot_index: number;
    /** 来源标识："P"=主链、"S"=备链、"-"=单路 */
    source: string;
    local_ip: string;
    local_port: number;
    remote_ip: string;
    remote_port: number;
    /** 原始数据十六进制（含前缀描述） */
    raw_hex: string;
    /** 提取/发送数据十六进制 */
    ext_hex: string;
}

/** UDP 数据事件（WebSocket 推送） */
export interface Ftj1cUdpDataEvent {
    type: "udp_data";
    connection_index: number;
    slot_index: number;
    source: string;
    local_ip: string;
    local_port: number;
    remote_ip: string;
    remote_port: number;
    raw_hex: string;
    ext_hex: string;
}

/** 联合类型（目前只有 udp_data 一种事件类型） */
export type Ftj1cEvent = Ftj1cUdpDataEvent;

/**
 * 创建 UDP 通信监控 API 对象
 *
 * @returns 包含所有通信监控接口的 API 对象
 */
export function createFtj1cApi() {
    const api = getFtj1c();
    return {
        /** 启动 UDP 监控服务 */
        async startService() {
            return api.ftj1cStartService();
        },

        /** 停止 UDP 监控服务 */
        async stopService() {
            return api.ftj1cStopService();
        },

        /** 查询服务运行状态 */
        async getServiceStatus() {
            return api.ftj1cGetServiceStatus();
        },

        /** 获取 IP 配置（16 组 IP:Port） */
        async getIpConfig() {
            return api.ftj1cGetIpConfig();
        },

        /** 读取配置文件内容 */
        async getConfig() {
            return api.ftj1cGetConfig();
        },

        /** 保存配置文件内容 */
        async saveConfig(content: string) {
            return api.ftj1cSaveConfig({content});
        },

        /** 获取用户操作说明（help_doc.md） */
        async getHelp() {
            return api.ftj1cGetHelp();
        },


        /**
         * 构建 WebSocket 地址（复用 shared 公共实现）
         *
         * 浏览器 WebSocket API 不支持自定义 header，
         * JWT token 通过 URL query 参数传递。
         */
        buildWebSocketUrl(): string {
            return sharedBuildWebSocketUrl("/api/ftj1c/ws");
        },
    };
}

/** UDP 通信监控 API 类型 */
export type Ftj1cApi = ReturnType<typeof createFtj1cApi>;
