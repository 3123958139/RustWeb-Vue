<!--
  发动机实时监控主界面（从 fj200c_information.informatization 的 App.vue 移植）

  布局与原桌面应用一致：
  - DataPanel：显示解码后的 SharedData 基本信息表格
  - CommandPanel：7 个命令通道（命令类型选择 + 小端十六进制数据 + 发送）
  - 发送 / 接收日志面板（各显示最近 2 条）
  - StatusBar：状态栏（连接状态 + 时钟）

  与原版差异：原版使用 Tauri 原生菜单（创建服务 / 打开配置 / 工具 / 帮助 /
  可视化 / 浏览数据），Web 版全部移到导航栏：
  - 创建服务：AppNavbar #actions 插槽的 ServiceNavButton（停止/启动服务）
  - 打开配置 / 工具 / 帮助：导航栏菜单项，各自独立路由页面
  - 可视化 / 浏览数据：导航栏菜单项（/fj200c_information/visual、/fj200c_information/data）
-->

<script lang="ts" setup>
/**
 * Composition API 核心用法：
 * - ref() 创建响应式基本类型
 * - computed() 创建计算属性
 * - 自定义组合式函数（useClock / useService / useCommandChannel / useFj200cInformationEvents）
 *   封装独立的业务逻辑，实现逻辑复用
 */
import {computed, ref} from "vue";
import {useClock} from "@/fj200c_information/composables/useClock";
import {useService} from "@/fj200c_information/composables/useService";
import {useCommandChannel} from "@/fj200c_information/composables/useCommandChannel";
import {useFj200cInformationEvents} from "@/fj200c_information/composables/useFj200cInformationEvents";
import DataPanel from "@/fj200c_information/components/DataPanel.vue";
import CommandPanel from "@/fj200c_information/components/CommandPanel.vue";
import StatusBar from "@/fj200c_information/components/StatusBar.vue";
import ServiceNavButton from "@/fj200c_information/components/ServiceNavButton.vue";
import {bytesToHex, hexStringToUint8Array} from "@/fj200c_information/utils/hex";
import {calculateChecksum} from "@/fj200c_information/utils/check";

/** 时钟组合式函数：每秒更新，提供 formatted() 方法返回格式化时间字符串 */
const clock = useClock();

// ========== 发送/接收日志 ==========
/** 接收日志（最新 200 条） */
const recvLog = ref<string[]>([]);
/** 发送日志（最新 200 条） */
const sendLog = ref<string[]>([]);
/** 帧计数器 */
const frameCount = ref(0);

/**
 * 添加发送日志条目
 * @param label - 通道标签（如 "1参数设置指令"）
 * @param msg   - 日志内容
 */
function pushSendLog(label: string, msg: string) {
  const timestamp = new Date().toLocaleTimeString();
  // unshift 将新条目插入数组头部（最新在前）
  sendLog.value.unshift(`[${timestamp}][${label}] ${msg}`);
  if (sendLog.value.length > 200) sendLog.value.length = 200; // 限制最大长度
}

// ========== 实时事件流 ==========
/**
 * WebSocket 事件流组合式函数
 *
 * onFrame 回调：每收到一帧解码数据时触发
 * - 更新帧计数器
 * - 将帧信息写入接收日志
 */
const events = useFj200cInformationEvents({
  onFrame: (_fields, hex, frameType) => {
    // 帧计数递增，到最大安全整数后归零
    frameCount.value = (frameCount.value + 1) % Number.MAX_SAFE_INTEGER;
    const timestamp = new Date().toLocaleTimeString();
    recvLog.value.unshift(`[${timestamp}][帧数${frameCount.value}][${frameType}] ${hex}`);
    if (recvLog.value.length > 200) recvLog.value.length = 200;
  },
});
const {connected, tableRows} = events;
events.connect(); // 建立 WebSocket 连接

/** 服务控制组合式函数：启动/停止发动机监控服务。
 *  WS 已连接时事件流即实时状态，仅在 WS 断开时以 3 秒轮询兜底 */
const {serviceRunning} = useService({isConnected: connected});

/** 状态描述文字（计算属性，serviceRunning 变化时自动更新） */
const statusText = computed(() =>
    serviceRunning.value ? "服务运行中..." : "系统就绪"
);

/** 连接状态文字 */
const connectionStatus = computed(() =>
    serviceRunning.value ? "已连接" : "未连接"
);

// ════════════════ 命令通道（从源 App.vue 的 channelConfigs 移植） ════════════════

/** 命令计数：每次发送时递增，写入命令帧的第 4 字节 */
const cmdCount = ref(0);

/**
 * 命令类型 → 数据区类型字节映射表
 * 用于构造命令帧时确定类型字段的值
 */
const TYPE_MAP: Record<string, number> = {
  产品名称: 0x1,
  发动机产品代号: 0x2,
  发动机出厂编号: 0x3,
  发动机检验试车日期: 0x4,
  电控器产品代号: 0x5,
  电控器编号: 0x6,
  燃气发生器编号: 0x7,
};

/** 命令类型下拉选项 */
const IDENTIFY_OPTIONS = [
  "产品名称",
  "发动机产品代号",
  "发动机出厂编号",
  "发动机检验试车日期",
  "电控器产品代号",
  "电控器编号",
  "燃气发生器编号",
];

/**
 * 构造 16 字节命令帧
 *
 * 帧结构：
 * [0-1] 前缀（EB 90 或 55 AA）
 * [2]   帧长度（0x10 = 16）
 * [3]   命令计数（每次递增）
 * [4]   类型字节
 * [5-14] 数据区
 * [15]  累加和校验
 *
 * @param prefix   - 2 字节前缀
 * @param typeByte - 命令类型字节
 * @param type     - 命令类型名称（用于查 TYPE_MAP）
 * @param input    - 用户输入的十六进制数据
 * @returns 十六进制字符串格式的完整命令帧
 */
function buildCmdFrame(
    prefix: [number, number],
    typeByte: number,
    type: string,
    cmdOptions: string[],
    input: string
): string {
  const byteData = new Uint8Array(16);
  byteData.set([
    prefix[0], prefix[1], 0x10, 0x00,
    0x00, 0x00, 0x00, typeByte,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
  ]);
  cmdCount.value = (cmdCount.value + 1) % 256;
  byteData[3] = cmdCount.value;
  let index = cmdOptions.findIndex((s) => s == type) ?? 0;
  if (typeByte == 0xDE)
    byteData[4] = index + 1;
  else
    byteData[4] = TYPE_MAP[type] ?? 0;
  console.log(typeByte);
  console.log(type);
  // 将用户输入的十六进制字符串解析为字节数组并填入帧
  const parsed = hexStringToUint8Array(input);
  byteData[5] = parsed[0] ?? 0;
  byteData[6] = parsed[1] ?? 0;
  byteData[8] = parsed[2] ?? 0;
  byteData[9] = parsed[3] ?? 0;
  byteData[10] = parsed[4] ?? 0;
  byteData[11] = parsed[5] ?? 0;
  byteData[12] = parsed[6] ?? 0;
  byteData[13] = parsed[7] ?? 0;
  // 最后一字节为累加和校验
  byteData[15] = calculateChecksum(byteData);
  return bytesToHex(byteData);
}

/** 命令通道配置项类型 */
interface ChannelConfigItem {
  label: string;
  disabledData: boolean;
  disabledType: boolean;
  cmdOptions: string[];
  transform: (cmdOptions: string[], type: string, input: string) => string;
}

/**
 * 7 个命令通道配置
 *
 * 每个通道对应一种命令类型：
 * 1. 参数设置指令（EB 90 + 0xEF）
 * 2. 参数读取指令（EB 90 + 0xED）
 * 3. 试验数据下载指令（55 AA + 0xDE）
 * 4. 试验数据首块（55 AA + 0xDC）
 * 5. 试验数据末块（55 AA + 0xDB）
 * 6. 基本参数清除指令（55 AA + 0xBF）
 * 7. 试验数据清除指令（55 AA + 0xBE）
 */
const channelConfigs: ChannelConfigItem[] = [
  {
    label: "1参数设置指令",
    disabledData: false,
    disabledType: false,
    cmdOptions: IDENTIFY_OPTIONS,
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xef, type, cmdOptions, input),
  },
  {
    label: "2参数读取指令",
    disabledData: false,
    disabledType: false,
    cmdOptions: IDENTIFY_OPTIONS,
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xed, type, cmdOptions, input),
  },
  {
    label: "3试验数据下载指令",
    disabledData: false,
    disabledType: false,
    cmdOptions: ["全部试验数据", "从上次读取位置开始下载数据"],
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xde, type, cmdOptions, input),
  },
  {
    label: "4试验数据首块",
    disabledData: true,
    disabledType: true,
    cmdOptions: [""],
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xdc, type, cmdOptions, input),
  },
  {
    label: "5试验数据末块",
    disabledData: true,
    disabledType: true,
    cmdOptions: [""],
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xdb, type, cmdOptions, input),
  },
  {
    label: "6基本参数清除指令",
    disabledData: true,
    disabledType: false,
    cmdOptions: IDENTIFY_OPTIONS,
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xbf, type, cmdOptions, input),
  },
  {
    label: "7试验数据清除指令",
    disabledData: true,
    disabledType: true,
    cmdOptions: [""],
    transform: (cmdOptions, type, input) =>
        buildCmdFrame([0xeb, 0x90], 0xbe, type, cmdOptions, input),
  },
];

/**
 * 初始化 7 个命令通道
 *
 * map() 遍历配置数组，为每个配置创建一个 useCommandChannel 实例。
 * 同时包装 sendCommand 方法，在发送后自动记录发送日志。
 */
const channels = channelConfigs.map((c) => {
  const ch = useCommandChannel({
    cmdOptions: c.cmdOptions,
    transform: c.transform,
  });
  // 包装原始 sendCommand，添加日志记录
  const origSend = ch.sendCommand;
  ch.sendCommand = async () => {
    await origSend();
    pushSendLog(
        c.label,
        ch.cmdLog.value[0] ||
        `[${ch.cmdType.value || "默认"}] ${ch.cmdData.value || "(内容为空)"}`
    );
  };
  return ch;
});

/** 通道标签数组（传递给 CommandPanel） */
const labels = channelConfigs.map((c) => c.label);
/** 各通道是否禁用类型选择 */
const disabledTypes = channelConfigs.map((c) => c.disabledType);
/** 各通道是否禁用数据输入 */
const disabledDatas = channelConfigs.map((c) => c.disabledData);
</script>

<template>
  <div class="fj200c_information-root">
    <div class="fj200c_information-page">
      <!-- 工具栏：标题 + 状态标签 + 时钟 -->
      <div class="fj200c_information-toolbar">
        <span class="toolbar-title">实时监控</span>
        <el-tag :type="serviceRunning ? 'success' : 'info'" size="small">
          {{ serviceRunning ? "服务运行中" : "服务已停止" }}
        </el-tag>
        <el-tag :type="connected ? 'success' : 'warning'" size="small">
          {{ connected ? "数据连接正常" : "数据连接中…" }}
        </el-tag>
        <div class="spacer"></div>
        <ServiceNavButton/>
        <span class="fj200c_information-clock">{{ clock.formatted() }}</span>
      </div>

      <!-- 主体：数据面板 + 命令面板 -->
      <div class="grid-row">
        <DataPanel :table-data="tableRows"/>
        <CommandPanel
            :channels="channels"
            :disabled-datas="disabledDatas"
            :disabled-types="disabledTypes"
            :labels="labels"
        />
      </div>

      <!-- 发送/接收日志（各显示最近 2 条） -->
      <div class="grid-row-sendrecv">
        <el-card class="grid-cell">
          <template #header>
            <div class="card-header">
              <span class="card-title">发送</span>
            </div>
          </template>
          <div class="log-area">
            <div v-for="(entry, i) in sendLog.slice(0, 2)" :key="i" class="log-entry">
              {{ entry }}
            </div>
          </div>
        </el-card>
        <el-card class="grid-cell">
          <template #header>
            <div class="card-header">
              <span class="card-title">接收</span>
            </div>
          </template>
          <div class="log-area">
            <div v-for="(entry, i) in recvLog.slice(0, 2)" :key="i" class="log-entry">
              {{ entry }}
            </div>
          </div>
        </el-card>
      </div>

      <!-- 底部状态栏 -->
      <StatusBar
          :connection-status="connectionStatus"
          :current-time="clock.formatted()"
          :status-text="statusText"
      />
    </div>
  </div>
</template>

<style scoped>
@import "@/fj200c_information/fj200c_information.css";

.grid-row {
  display: flex;
  gap: 8px;
  height: 520px;
  margin-bottom: 16px;
}

.grid-row-sendrecv {
  display: flex;
  gap: 8px;
  height: 260px;
  margin-bottom: 16px;
}

.grid-row-sendrecv .grid-cell {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.grid-row-sendrecv .grid-cell :deep(.el-card__body) {
  flex: 1;
  overflow: hidden;
  padding: 6px 10px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
}

.log-area {
  height: 100%;
  overflow-y: auto;
  font-family: "Courier New", Courier, monospace;
  font-size: 12px;
  line-height: 1.6;
}

.log-entry {
  padding: 2px 4px;
  border-bottom: 1px solid #f0f0f0;
  word-break: break-all;
}

.log-entry:last-child {
  border-bottom: none;
}

.spacer {
  flex: 1;
}
</style>
