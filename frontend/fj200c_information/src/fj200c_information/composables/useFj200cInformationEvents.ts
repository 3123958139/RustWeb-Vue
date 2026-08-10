/**
 * 发动机模块 WebSocket 事件流组合式函数
 *
 * 替代原 Tauri 的 3 类事件监听（frame / payload / table_data）。
 * 单个页面共享一条 WebSocket 连接，自动断开重连。
 *
 * 特性：
 * - 断开自动重连（1.5 秒间隔）
 * - 事件按 type 字段分发到对应回调
 * - 组件卸载时自动断开连接
 */

import { onUnmounted, ref } from "vue";
import { fj200c_informationApi } from "@/api";
import type { Fj200cInformationEvent, TableRow } from "@/fj200c_information/api/fj200c_information";

/** 载荷日志条目 */
export interface PayloadItem {
  time: string;
  hex: string;
}

/** 事件回调配置 */
export interface UseFj200cInformationEventsOptions {
  /** 每收到一帧数据时回调（可视化图表驱动） */
  onFrame?: (fields: string[], hex: string, frameType: string) => void;
  /** 收到原始数据时回调 */
  onPayload?: (item: PayloadItem) => void;
}

/**
 * WebSocket 事件流组合式函数
 *
 * @param options - 事件回调配置
 * @returns 包含连接状态、数据和控制方法的对象
 */
export function useFj200cInformationEvents(options: UseFj200cInformationEventsOptions = {}) {
  /** WebSocket 是否已连接 */
  const connected = ref(false);
  /** 正在建立连接 */
  const connecting = ref(false);
  /** 原始载荷日志（最新 200 条） */
  const payloadLog = ref<PayloadItem[]>([]);
  /** 解码后的表格数据 */
  const tableRows = ref<TableRow[]>([]);
  /** 最新帧的十六进制数据 */
  const lastFrameHex = ref("");
  /** 最新帧的类型 */
  const lastFrameType = ref("");
  /** 最新帧的解码字段数组 */
  const lastFrameFields = ref<string[]>([]);

  /** WebSocket 实例 */
  let ws: WebSocket | null = null;
  /** 重连定时器 */
  let reconnectTimer: number | null = null;
  /** 是否为手动关闭（手动关闭时不自动重连） */
  let manualClose = false;

  /**
   * 格式化时间为 "HH:mm:ss" 格式
   * @param d - Date 对象
   */
  const padTime = (d: Date) => {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  };

  /**
   * 处理 WebSocket 事件
   *
   * 根据事件 type 字段分发到不同的处理逻辑：
   * - payload: 追加到载荷日志
   * - table_data: 更新表格数据
   * - frame: 更新帧信息并触发回调
   */
  const handleEvent = (event: Fj200cInformationEvent) => {
    switch (event.type) {
      case "payload":
        const item: PayloadItem = {
          time: padTime(new Date()),
          hex: event.hex,
        };
        payloadLog.value.unshift(item);
        if (payloadLog.value.length > 200) {
          payloadLog.value.length = 200; // 原地截断，避免每帧 slice 整体替换
        }
        options.onPayload?.(item); // 可选链：有回调才调用
        break;
      case "table_data":
        tableRows.value = event.rows;
        break;
      case "frame":
        lastFrameHex.value = event.hex;
        lastFrameType.value = event.frame_type;
        lastFrameFields.value = event.fields;
        options.onFrame?.(event.fields, event.hex, event.frame_type);
        break;
    }
  };

  /**
   * 建立 WebSocket 连接
   *
   * 使用 fj200c_informationApi.buildWebSocketUrl() 构建带 JWT token 的 URL。
   * 浏览器 WebSocket API 不支持自定义 header，
   * 因此 JWT 通过 URL query 参数传递。
   */
  const connect = () => {
    if (ws || connecting.value) return; // 避免重复连接
    manualClose = false;
    connecting.value = true;

    ws = new WebSocket(fj200c_informationApi.buildWebSocketUrl());

    ws.onopen = () => {
      connected.value = true;
      connecting.value = false;
    };

    ws.onmessage = (message) => {
      try {
        const data = JSON.parse(message.data) as Fj200cInformationEvent;
        handleEvent(data);
      } catch {
        // 忽略无法解析的消息（非 JSON 格式或空消息）
      }
    };

    ws.onclose = () => {
      connected.value = false;
      connecting.value = false;
      ws = null;
      // 非手动关闭时，1.5 秒后自动重连
      if (!manualClose) {
        reconnectTimer = window.setTimeout(connect, 1500);
      }
    };

    ws.onerror = () => {
      ws?.close(); // 错误时关闭连接，触发 onclose 重连逻辑
    };
  };

  /**
   * 手动断开 WebSocket 连接
   * 设置 manualClose 阻止自动重连
   */
  const disconnect = () => {
    manualClose = true;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    ws?.close();
    ws = null;
    connected.value = false;
  };

  /** 清空载荷日志 */
  const clearPayload = () => {
    payloadLog.value = [];
  };

  // 组件卸载时自动断开连接
  onUnmounted(disconnect);

  return {
    connected,
    connecting,
    payloadLog,
    tableRows,
    lastFrameHex,
    lastFrameType,
    lastFrameFields,
    connect,
    disconnect,
    clearPayload,
  };
}
