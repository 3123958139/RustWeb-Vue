<!--
  UDP 通信监控面板（ftj1c）

  从 demo-test3-ftj 的 App.vue 迁移，采用 4x4 卡片网格布局。

  功能：
  1. 服务控制：启动/停止 UDP 监控服务
  2. 4x4 卡片网格：8 路连接 x 2 张卡片（原始数据 + 提取数据）
  3. WebSocket 实时接收 UDP 数据并更新卡片
  4. 配置编辑：读取/保存 config-ftj1c.ini
  5. IP 配置：从后端获取 16 组 IP:Port 填充卡片标题
  6. 状态栏：显示连接状态、当前时间、错误信息
-->
<template>
  <div class="ftj1c-root">
    <!-- 全局导航条由 App.vue 渲染（登录页除外） -->

    <div class="ftj1c-page">
      <!-- 工具栏：标题 + 状态标签 + 操作按钮 -->
      <div class="ftj1c-toolbar">
        <span class="toolbar-title">UDP 通信监控</span>
        <el-tag :type="serviceRunning ? 'success' : 'info'" size="small">
          {{ serviceRunning ? "服务运行中" : "服务已停止" }}
        </el-tag>
        <el-tag :type="connected ? 'success' : 'warning'" size="small">
          {{ connected ? "数据连接正常" : "数据连接中…" }}
        </el-tag>
        <el-tag size="small" type="info">{{ mockMode ? "模拟模式" : "真实模式" }}</el-tag>
        <div class="spacer"></div>
        <span class="ftj1c-clock">{{ currentTime }}</span>
        <el-button type="primary" size="small" :loading="starting || stopping" @click="onToggleService">
          {{ serviceRunning ? "停止服务" : "启动服务" }}
        </el-button>
        <el-button size="small" @click="openConfig">配置</el-button>
      </div>

      <!-- 4x4 卡片网格（8 路连接 x 2 张卡片 = 16 格） -->
      <div class="grid-container">
        <div v-for="row in 4" :key="row" class="grid-row">
          <el-card
            v-for="col in 4"
            :key="(row - 1) * 4 + col"
            class="grid-cell"
            :class="col % 2 === 1 ? 'col-odd' : 'col-even'"
          >
            <template #header>
              <div class="card-header">
                <span class="card-title">{{ cells[(row - 1) * 4 + (col - 1)].title.split("  ")[0] }}</span>
                <span class="card-addr">{{ cells[(row - 1) * 4 + (col - 1)].title.split("  ")[1] }}</span>
              </div>
            </template>
            <p>{{ cells[(row - 1) * 4 + (col - 1)].content || "等待数据…" }}</p>
          </el-card>
        </div>
      </div>

      <!-- 底部状态栏 -->
      <div class="status-bar">
        <div class="status-bar-left">
          <span
            class="status-indicator"
            :class="connectionStatus === '已连接' ? 'connected' : 'disconnected'"
          ></span>
          <span class="status-item">状态: {{ statusText }}</span>
          <span class="status-divider">|</span>
          <span class="status-item">连接: {{ connectionStatus }}</span>
        </div>
        <div class="status-bar-right">
          <span v-if="errorMessage" class="status-item status-error">{{ errorMessage }}</span>
          <span class="status-item">{{ currentTime }}</span>
        </div>
      </div>
    </div>

    <!-- 配置编辑对话框 -->
    <el-dialog
      v-model="configVisible"
      title="配置文件 (config-ftj1c.ini)"
      width="860px"
      top="5vh"
      @close="handleConfigCancel"
    >
      <el-input
        v-model="configContent"
        type="textarea"
        :rows="28"
        spellcheck="false"
        style="font-family: 'Consolas', 'Courier New', monospace; font-size: 13px"
      />
      <template #footer>
        <el-button @click="handleConfigCancel">取消</el-button>
        <el-button type="primary" :loading="savingConfig" @click="handleConfigSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { ftj1cApi } from "@/api";
import type { Ftj1cEvent, IpConfig } from "@/ftj1c/api/ftj1c";

// ========== 服务控制 ==========

/** 服务是否正在运行 */
const serviceRunning = ref(false);
/** 启动操作进行中 */
const starting = ref(false);
/** 停止操作进行中 */
const stopping = ref(false);
/** 状态描述文字 */
const statusText = ref("系统就绪");
/** 连接状态 */
const connectionStatus = ref("未连接");
/** 是否为模拟模式（从配置文件读取） */
const mockMode = ref(true);
/** 错误信息 */
const errorMessage = ref("");

/**
 * 加载服务运行状态
 * 调用后端 API 查询，失败时静默忽略（由路由守卫处理未登录场景）
 */
async function loadServiceStatus() {
  try {
    const response = await ftj1cApi.getServiceStatus();
    serviceRunning.value = response.data?.running ?? false;
  } catch {
    // 忽略错误
  }
}

/**
 * 加载模拟模式开关
 * 从 config-ftj1c.ini 内容中解析 [Udp] Mock = true/false
 */
async function loadMockMode() {
  try {
    const response = await ftj1cApi.getConfig();
    const content = response.data?.content ?? "";
    // 使用正则匹配配置行
    const match = content.match(/^\s*Mock\s*=\s*(true|false)\s*$/im);
    if (match) {
      mockMode.value = match[1].toLowerCase() === "true";
    }
  } catch {
    // 读取失败保持默认值
  }
}

/**
 * 切换服务状态（启动/停止）
 */
async function onToggleService() {
  if (serviceRunning.value) {
    stopping.value = true;
    try {
      await ftj1cApi.stopService();
      serviceRunning.value = false;
      statusText.value = "服务已停止";
      connectionStatus.value = "未连接";
      ElMessage.success("服务已停止");
    } catch (e: any) {
      ElMessage.error(e?.response?.data?.message || "停止服务失败");
    } finally {
      stopping.value = false;
    }
  } else {
    starting.value = true;
    try {
      await ftj1cApi.startService();
      serviceRunning.value = true;
      statusText.value = "服务运行中...";
      connectionStatus.value = "已连接";
      ElMessage.success("服务已启动");
    } catch (e: any) {
      ElMessage.error(e?.response?.data?.message || "启动服务失败");
    } finally {
      starting.value = false;
    }
  }
}

// ========== 配置对话框 ==========

/** 配置对话框是否可见 */
const configVisible = ref(false);
/** 配置文件内容（可编辑） */
const configContent = ref("");
/** 保存加载状态 */
const savingConfig = ref(false);
/** 保存前的配置内容（用于取消时恢复） */
let savedConfig = "";

/** 打开配置对话框并加载文件内容 */
async function openConfig() {
  try {
    const response = await ftj1cApi.getConfig();
    configContent.value = response.data?.content ?? "";
    savedConfig = configContent.value;
    configVisible.value = true;
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "读取配置失败");
  }
}

/** 保存配置文件 */
async function handleConfigSave() {
  savingConfig.value = true;
  try {
    await ftj1cApi.saveConfig(configContent.value);
    savedConfig = configContent.value;
    configVisible.value = false;
    ElMessage.success("配置已保存（重启服务后生效）");
  } catch (e: any) {
    ElMessage.error(e?.response?.data?.message || "保存配置失败");
  } finally {
    savingConfig.value = false;
  }
}

/** 取消编辑（恢复原始内容） */
function handleConfigCancel() {
  configContent.value = savedConfig;
  configVisible.value = false;
}

// ========== 卡片网格（8 路连接 x 2 张卡片 = 16 格） ==========

/**
 * 连接映射配置
 * 每路连接对应两个 IP 索引（源/目的），标题描述数据流向
 */
const connectionMap = [
  { ipIndices: [1, 2], title: "转换模块->基地（遥测帧IP2->IP1）" },
  { ipIndices: [3, 4], title: "转换模块->基地（链监帧IP4->IP3）" },
  { ipIndices: [5, 6], title: "基地->转换模块（引导帧IP5->IP6）" },
  { ipIndices: [7, 8], title: "基地->转换模块（链监帧IP7->IP8）" },
  { ipIndices: [9, 10], title: "转换模块->靶机（载荷帧IP10->IP9）" },
  { ipIndices: [11, 12], title: "转换模块->靶机（（主链）飞控帧IP11->IP12）" },
  { ipIndices: [13, 14], title: "转换模块->靶机（载机帧IP14->IP13）" },
  { ipIndices: [15, 16], title: "转换模块->靶机（（副链）飞控帧IP15->IP16）" },
];

/** 卡片数据类型 */
interface Cell {
  title: string;
  content: string;
}

/** 16 个卡片的数据（标题 + 内容） */
const cells = ref<Cell[]>(Array.from({ length: 16 }, () => ({ title: "", content: "" })));

/**
 * 根据 IP 编号格式化 IP:Port 字符串
 * @param ipConfig - IP 配置对象
 * @param ipNum    - IP 编号（1~16）
 */
function formatIpAddr(ipConfig: IpConfig, ipNum: number): string {
  const ip = (ipConfig as any)[`ip${ipNum}`] || "";
  const port = (ipConfig as any)[`port${ipNum}`];
  return port ? `${ip}:${port}` : ip;
}

/**
 * 加载 IP 配置并填充卡片标题
 * 根据连接映射配置，将 IP:Port 填入对应卡片的标题中
 */
async function loadIpConfig() {
  try {
    const response = await ftj1cApi.getIpConfig();
    const ipConfig = response.data ?? {};
    connectionMap.forEach((conn, idx) => {
      // 标题中第一个数字决定源/目的映射
      const firstNum = parseInt(conn.title.match(/\d+/)?.[0] ?? "", 10);
      const even = firstNum % 2 === 0;
      const addrA = formatIpAddr(ipConfig, even ? conn.ipIndices[1] : conn.ipIndices[0]);
      const addrB = formatIpAddr(ipConfig, even ? conn.ipIndices[0] : conn.ipIndices[1]);
      cells.value[idx * 2] = { title: `${conn.title} [源]  ${addrA}`, content: "" };
      cells.value[idx * 2 + 1] = { title: `${conn.title} [目的]  ${addrB}`, content: "" };
    });
    statusText.value = "配置已加载";
  } catch {
    statusText.value = "配置加载失败";
  }
}

// ========== WebSocket 事件流 ==========

/** WebSocket 是否已连接 */
const connected = ref(false);

/** WebSocket 实例 */
let ws: WebSocket | null = null;
/** 重连定时器 */
let reconnectTimer: number | null = null;
/** 是否为手动关闭 */
let manualClose = false;

/**
 * 建立 WebSocket 连接
 *
 * 接收 udp_data 事件后，根据 connection_index 更新对应卡片的内容：
 * - 偶数索引卡片：原始数据（raw_hex）
 * - 奇数索引卡片：提取数据（ext_hex）
 */
function connectWs() {
  if (ws || manualClose) return;
  ws = new WebSocket(ftj1cApi.buildWebSocketUrl());

  ws.onopen = () => {
    connected.value = true;
  };

  ws.onmessage = (message) => {
    try {
      const data = JSON.parse(message.data) as Ftj1cEvent;
      if (data.type !== "udp_data") return;
      const { connection_index, raw_hex, ext_hex } = data;
      // connection_index 0~7 映射到卡片索引 0~15（每路连接 2 张卡片）
      const rawIdx = connection_index * 2;
      const extIdx = connection_index * 2 + 1;
      if (rawIdx < cells.value.length && extIdx < cells.value.length) {
        const now = new Date().toLocaleString();
        cells.value[rawIdx] = { ...cells.value[rawIdx], content: `【${now}】${raw_hex}` };
        cells.value[extIdx] = { ...cells.value[extIdx], content: `【${now}】${ext_hex}` };
      }
    } catch {
      // 忽略无法解析的消息
    }
  };

  ws.onclose = () => {
    connected.value = false;
    ws = null;
    // 非手动关闭时，1.5 秒后自动重连
    if (!manualClose) {
      reconnectTimer = window.setTimeout(connectWs, 1500);
    }
  };

  ws.onerror = () => {
    ws?.close();
  };
}

/** 手动断开 WebSocket 连接 */
function disconnectWs() {
  manualClose = true;
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  ws?.close();
  ws = null;
  connected.value = false;
}

// ========== 时钟 ==========

/** 当前时间字符串 */
const currentTime = ref(new Date().toLocaleString("zh-CN", { hour12: false }));
/** 时钟定时器 */
let timerInterval: ReturnType<typeof setInterval> | null = null;

/**
 * 组件挂载时初始化
 *
 * 执行顺序：
 * 1. 加载服务状态
 * 2. 加载模拟模式配置
 * 3. 加载 IP 配置
 * 4. 建立 WebSocket 连接
 * 5. 启动时钟
 */
onMounted(async () => {
  await loadServiceStatus();
  await loadMockMode();
  await loadIpConfig();
  connectWs();
  timerInterval = setInterval(() => {
    currentTime.value = new Date().toLocaleString("zh-CN", { hour12: false });
  }, 1000);
});

/**
 * 组件卸载时清理资源
 * 断开 WebSocket、清除定时器，防止内存泄漏
 */
onUnmounted(() => {
  disconnectWs();
  if (timerInterval) {
    clearInterval(timerInterval);
  }
});
</script>

<style scoped>
.ftj1c-root {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
}

.ftj1c-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  flex: 1;
  height: calc(100vh - 64px);
  box-sizing: border-box;
  min-height: 0;
}

.ftj1c-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.toolbar-title {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.spacer { flex: 1; }

.ftj1c-clock {
  font-size: 13px;
  color: #909399;
  white-space: nowrap;
}

.grid-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.grid-row {
  display: flex;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.grid-cell {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.grid-cell :deep(.el-card__body) {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

.grid-cell :deep(.el-card__header) {
  color: #fff;
  padding: 6px 12px;
}

.grid-cell.col-odd :deep(.el-card__header) {
  background-color: #409eff;
}

.grid-cell.col-even :deep(.el-card__header) {
  background-color: #1999aa;
}

.card-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.card-title {
  font-weight: bold;
  font-size: 12px;
  line-height: 1.3;
}

.card-addr {
  font-weight: normal;
  font-size: 10px;
  line-height: 1.2;
  opacity: 0.85;
}

.grid-cell p {
  white-space: pre-wrap;
  margin: 0;
  line-height: 1.4;
  font-size: 11px;
  word-break: break-all;
}

.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 28px;
  padding: 0 12px;
  background-color: #303133;
  color: #e0e0e0;
  font-size: 12px;
  border-radius: 4px;
  flex-shrink: 0;
}

.status-bar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-bar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-item {
  white-space: nowrap;
}

.status-divider {
  color: #606266;
}

.status-error {
  color: #f56c6c;
}

.status-indicator {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-indicator.connected {
  background-color: #1999aa;
  box-shadow: 0 0 4px #1999aa;
}

.status-indicator.disconnected {
  background-color: #909399;
}
</style>
