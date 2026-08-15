/**
 * 飞控地面站 WebSocket 事件流组合式函数
 *
 * 单页面共享一条 WebSocket 连接，自动断开重连，按 `event` 字段分发：
 * - telemetry：10Hz 遥测快照（内部标签序列化，扁平字段 + event 标记）
 * - mission_progress：任务上传/下载/清除进度与结果
 * - command_ack：命令回执
 *
 * 特性：
 * - 断开自动重连（1.5 秒间隔）
 * - 组件卸载时自动断开连接
 */

import { onUnmounted, ref } from "vue";
import { qgcApi } from "@/api";
import type { QgcTelemetry, QgcWsEvent } from "@/qgc/api/qgc";

/** 事件回调配置 */
export interface UseQgcEventsOptions {
  /** 收到遥测事件时回调 */
  onTelemetry?: (t: Partial<QgcTelemetry>) => void;
  /** 收到任务进度事件时回调 */
  onMissionProgress?: (p: Extract<QgcWsEvent, { event: "mission_progress" }>) => void;
  /** 收到命令回执事件时回调 */
  onCommandAck?: (a: Extract<QgcWsEvent, { event: "command_ack" }>) => void;
}

/**
 * WebSocket 事件流组合式函数
 *
 * @param options - 事件回调配置
 * @returns 包含连接状态和控制方法的对象
 */
export function useQgcEvents(options: UseQgcEventsOptions = {}) {
  /** WebSocket 是否已连接 */
  const connected = ref(false);
  /** 正在建立连接 */
  const connecting = ref(false);
  /** 最新遥测快照（扁平字段，内部标签已剔除） */
  const telemetry = ref<Partial<QgcTelemetry>>({});
  /** 最近一次任务进度事件 */
  const lastMissionProgress = ref<Extract<QgcWsEvent, { event: "mission_progress" }> | null>(null);
  /** 最近一次命令回执事件 */
  const lastCommandAck = ref<Extract<QgcWsEvent, { event: "command_ack" }> | null>(null);

  /** WebSocket 实例 */
  let ws: WebSocket | null = null;
  /** 重连定时器 */
  let reconnectTimer: number | null = null;
  /** 是否为手动关闭（手动关闭时不自动重连） */
  let manualClose = false;

  /**
   * 处理 WebSocket 事件
   *
   * 后端使用 `#[serde(tag = "event")]` 内部标签序列化，
   * 事件对象为 `{ event: "telemetry", ...遥测字段 }` 扁平结构。
   */
  const handleEvent = (event: QgcWsEvent) => {
    switch (event.event) {
      case "telemetry":
        // 剔除 event 标记后整体替换遥测快照
        const { event: _tag, ...rest } = event;
        telemetry.value = rest;
        options.onTelemetry?.(rest);
        break;
      case "mission_progress":
        lastMissionProgress.value = event;
        options.onMissionProgress?.(event);
        break;
      case "command_ack":
        lastCommandAck.value = event;
        options.onCommandAck?.(event);
        break;
    }
  };

  /**
   * 建立 WebSocket 连接
   *
   * 使用 qgcApi.buildWebSocketUrl() 构建带 JWT token 的 URL。
   * 浏览器 WebSocket API 不支持自定义 header，
   * 因此 JWT 通过 URL query 参数传递。
   */
  const connect = () => {
    if (ws || connecting.value) return; // 避免重复连接
    manualClose = false;
    connecting.value = true;

    ws = new WebSocket(qgcApi.buildWebSocketUrl());

    ws.onopen = () => {
      connected.value = true;
      connecting.value = false;
    };

    ws.onmessage = (message) => {
      try {
        const data = JSON.parse(message.data) as QgcWsEvent;
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

  // 组件卸载时自动断开连接
  onUnmounted(disconnect);

  return {
    connected,
    connecting,
    telemetry,
    lastMissionProgress,
    lastCommandAck,
    connect,
    disconnect,
  };
}
