/**
 * 飞控告警检测与语音播报组合式函数
 *
 * 基于遥测快照做阈值检测（低电量 / 飞控失联 / 高度超限 / GPS 弱），
 * 输出当前激活告警列表供大屏高亮，并通过 Web Speech API 对严重告警
 * 做语音播报（节流避免刷屏）。检测纯前端完成，不依赖后端。
 *
 * 用法：
 * ```ts
 * const { alarms, speechEnabled, analyze } = useQgcAlarms();
 * useQgcEvents({ onTelemetry: (t) => analyze(t) });
 * ```
 */

import { ref } from "vue";
import type { QgcTelemetry } from "@/qgc/api/qgc";

/** 单条告警 */
export interface AlarmItem {
  /** 告警标识（同类型稳定，用于去重/节流） */
  id: string;
  /** 等级：warn 警告 / critical 严重 */
  level: "warn" | "critical";
  /** 简短标签 */
  label: string;
  /** 完整描述（用于横幅与语音播报） */
  message: string;
}

/** 告警阈值（可配置） */
export interface AlarmThresholds {
  /** 电量低于此值告警（%） */
  batteryWarn: number;
  /** 电量低于此值严重告警（%） */
  batteryCritical: number;
  /** 相对高度上限（米） */
  maxAltitude: number;
}

const DEFAULT_THRESHOLDS: AlarmThresholds = {
  batteryWarn: 20,
  batteryCritical: 10,
  maxAltitude: 120,
};

export interface UseQgcAlarmsOptions {
  thresholds?: Partial<AlarmThresholds>;
}

/**
 * 告警检测与语音播报
 *
 * @param options - 可选阈值覆盖
 * @returns 当前告警列表、语音开关、以及每帧调用的 `analyze`
 */
export function useQgcAlarms(options: UseQgcAlarmsOptions = {}) {
  const thresholds: AlarmThresholds = { ...DEFAULT_THRESHOLDS, ...options.thresholds };

  /** 当前激活告警（按 severity 排序，critical 在前） */
  const alarms = ref<AlarmItem[]>([]);
  /** 语音播报开关 */
  const speechEnabled = ref(true);

  /** 是否曾与飞控建立连接（用于区分「从未连接」与「失联」） */
  let hadConnected = false;
  /** 上次播报时间戳（按文本节流，避免重复刷屏） */
  const lastSpoken: Record<string, number> = {};

  /**
   * 语音播报（中文）
   *
   * 同一文本 15 秒内最多播报一次；浏览器不支持或开关关闭时静默。
   */
  function speak(text: string) {
    if (!speechEnabled.value) return;
    const w = window as unknown as { speechSynthesis?: SpeechSynthesis };
    if (!w.speechSynthesis) return;
    const now = Date.now();
    if (lastSpoken[text] && now - lastSpoken[text] < 15000) return;
    lastSpoken[text] = now;
    try {
      w.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = "zh-CN";
      u.rate = 1.1;
      w.speechSynthesis.speak(u);
    } catch {
      // 语音引擎不可用时忽略
    }
  }

  /**
   * 逐帧分析遥测，更新激活告警列表
   *
   * @param t - 遥测快照（扁平字段，来自 WebSocket telemetry 事件）
   */
  function analyze(t: Partial<QgcTelemetry>) {
    const next: AlarmItem[] = [];

    // 低电量（battery_remaining = -1 表示未知，不告警）
    const bat = t.battery_remaining ?? -1;
    if (bat >= 0) {
      if (bat <= thresholds.batteryCritical) {
        next.push({ id: "battery", level: "critical", label: "低电量", message: `电池电量 ${bat}%，请立即返航` });
      } else if (bat <= thresholds.batteryWarn) {
        next.push({ id: "battery", level: "warn", label: "电量偏低", message: `电池电量 ${bat}%` });
      }
    }

    // 飞控失联：曾连接后中断才告警（避免未起飞时持续误报）
    if (t.connected) {
      hadConnected = true;
    } else if (hadConnected) {
      next.push({ id: "link", level: "critical", label: "飞控失联", message: "与飞控的通信已中断" });
    }

    // 高度超限
    const alt = t.relative_alt ?? 0;
    if (alt > thresholds.maxAltitude) {
      next.push({ id: "alt", level: "warn", label: "高度超限", message: `当前高度 ${alt.toFixed(0)} 米，超过 ${thresholds.maxAltitude} 米限制` });
    }

    // GPS 弱（已连接但未达 3D 定位）
    if (t.connected && (t.gps_fix_type ?? 0) < 3) {
      next.push({ id: "gps", level: "warn", label: "GPS 弱", message: "卫星定位不足，未达 3D 定位" });
    }

    // 遥控器丢失 / 信号弱（rc_rssi=0 为丢失，<40 为弱；无 RC 数据时（未连接）不告警）
    const rssi = t.rc_rssi ?? 0;
    const hasRc = t.connected && (t.rc_channels?.length ?? 0) > 0;
    if (hasRc) {
      if (rssi === 0) {
        next.push({ id: "rc", level: "critical", label: "遥控器丢失", message: "遥控器信号丢失，启用失控保护" });
      } else if (rssi < 40) {
        next.push({ id: "rc", level: "warn", label: "遥控信号弱", message: `遥控器信号弱（${rssi}）` });
      }
    }

    // critical 优先排序
    next.sort((a, b) => (a.level === b.level ? 0 : a.level === "critical" ? -1 : 1));
    alarms.value = next;

    // 新出现的严重告警触发语音播报
    for (const a of next) {
      if (a.level === "critical") speak(a.message);
    }
  }

  return { alarms, speechEnabled, analyze };
}
