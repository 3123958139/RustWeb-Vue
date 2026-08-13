//! # 后端串口数据流组合式函数（WebSocket 版）
//!
//! 替代原 Tauri 的 4 类事件监听（port_data / simulation_state / theme_toggle /
//! csv_recording_state）。WebSocket 连接为**模块级单例**，通过引用计数共享：
//! App.vue 挂载时建立应用级常驻连接，页面切换不会断连，
//! 任何页面（含试验数据查看页 ExperimentView）都实时收到 port_data。
//!
//! 特性：
//! - 断开自动重连（1.5 秒间隔）
//! - 未登录（无 token）时不发起连接，等待下次重连时机
//! - 连接建立时先收到一个 JSON 数组（5 个 PortData 快照），之后为单个事件对象
//! - 按 `type` 字段分发到 dashboard store / useTheme
//! - 组件卸载时释放引用；引用归零时才真正断开连接
//!
//! 使用方式：
//! - App.vue 调用 `useBackendPorts()`：应用级连接，从不断开
//! - Monitor.vue 调用 `useBackendPorts()`：页面级引用，仅增加引用计数

import {onMounted, onUnmounted} from 'vue'
import {getSessionToken} from '@shared'
import {useDashboardStore} from '../store/dashboard'
import {fj200cMainApi} from '@/api'
import type {Fj200cMainWsEvent, PortDataEvent} from '@/fj200c_main/api/fj200c_main'
import type {Adam4015Fields, Adam4117Fields, DynoFields, EcuFields, FluxFields} from '@shared/api/generated'
import {applyTheme} from './useTheme'

const adam4117ParamIndices = [0, 1, 2]

// ---- 模块级共享连接状态（引用计数） ----
let sharedWs: WebSocket | null = null
let reconnectTimer: number | null = null
let manualClose = false
let refCount = 0

export function useBackendPorts() {
    const store = useDashboardStore()

    function handlePortData(event: PortDataEvent) {
        const {connection_index, fields, hex} = event
        switch (connection_index) {
            case 0:
                if ('Ecu' in fields) handleEcu(store, fields.Ecu, hex)
                break
            case 1:
                if ('Adam4015' in fields) handleAdam4015(store, fields.Adam4015, hex)
                break
            case 2:
                if ('Adam4117' in fields) handleAdam4117(store, fields.Adam4117, hex)
                break
            case 3:
                if ('Dyno' in fields) handleDyno(store, fields.Dyno, hex)
                break
            case 4:
                if ('Flux' in fields) handleFlux(store, fields.Flux, hex)
                break
        }
    }

    function handleEvent(event: Fj200cMainWsEvent) {
        switch (event.type) {
            case 'port_data':
                handlePortData(event)
                break
            case 'simulation_state':
                store.isSimulating = event.simulating
                break
            case 'theme_state':
                applyTheme(event.isDark)
                break
            case 'csv_recording_state':
                store.isRecording = event.recording
                break
        }
    }

    function handleMessage(data: unknown) {
        // 连接建立时先收到一个 JSON 数组（5 个 PortData 快照），之后为单个事件对象
        if (Array.isArray(data)) {
            for (const item of data) handleEvent(item as Fj200cMainWsEvent)
        } else {
            handleEvent(data as Fj200cMainWsEvent)
        }
    }

    function connect() {
        if (sharedWs || manualClose) return
        // 未登录时没有有效 token，跳过连接，等待重连定时器触发
        if (!getSessionToken()) {
            scheduleReconnect()
            return
        }
        let ws: WebSocket | null = null
        try {
            ws = new WebSocket(fj200cMainApi.buildWebSocketUrl())
        } catch (e) {
            console.warn('[useBackendPorts] WebSocket 创建失败:', e)
            scheduleReconnect()
            return
        }
        sharedWs = ws

        ws.onmessage = (message) => {
            try {
                const data = JSON.parse(message.data)
                handleMessage(data)
            } catch {
                // 忽略无法解析的消息
            }
        }

        ws.onclose = () => {
            if (sharedWs === ws) sharedWs = null
            if (!manualClose) scheduleReconnect()
        }

        ws.onerror = () => {
            ws?.close()
        }
    }

    function scheduleReconnect() {
        if (manualClose) return
        if (reconnectTimer) return
        reconnectTimer = window.setTimeout(() => {
            reconnectTimer = null
            connect()
        }, 1500)
    }

    function acquire() {
        refCount++
        manualClose = false
        connect()
    }

    function release() {
        refCount = Math.max(0, refCount - 1)
        if (refCount > 0) return
        manualClose = true
        if (reconnectTimer) {
            clearTimeout(reconnectTimer)
            reconnectTimer = null
        }
        sharedWs?.close()
        sharedWs = null
    }

    onMounted(acquire)
    onUnmounted(release)
}

function handleEcu(store: ReturnType<typeof useDashboardStore>, f: EcuFields, hex: string) {
    store.$patch((state) => {
        Object.assign(state.ecuData, f)
    })
    store.footerStats.ecuRxBytes += Math.floor(hex.length / 2)
    store.footerStats.ecuRxFrames++
}

function handleAdam4015(store: ReturnType<typeof useDashboardStore>, f: Adam4015Fields, hex: string) {
    store.$patch((state) => {
        state.envParams[3].value = f.channels[3] ?? 0
    })
    store.footerStats.adam4015RxBytes += hex.length / 2
}

function handleAdam4117(store: ReturnType<typeof useDashboardStore>, f: Adam4117Fields, hex: string) {
    store.$patch((state) => {
        for (const i of adam4117ParamIndices) {
            state.envParams[i].value = f.channels[i] ?? 0
        }
    })
    store.footerStats.adam4117RxBytes += hex.length / 2
}

function handleDyno(store: ReturnType<typeof useDashboardStore>, f: DynoFields, hex: string) {
    store.$patch((state) => {
        state.dynoData.jkwd = f.jkwd ?? 0
        state.dynoData.njzs = f.njzs ?? 0
        state.dynoData.nj = f.nj ?? 0
        state.dynoData.njgl = f.njgl ?? 0
        state.envParams[5].value = f.njzs ?? 0
        state.envParams[6].value = f.nj ?? 0
        state.envParams[7].value = f.njgl ?? 0
    })
    store.footerStats.dynoRxBytes += hex.length / 2
}

function handleFlux(store: ReturnType<typeof useDashboardStore>, f: FluxFields, hex: string) {
    store.$patch((state) => {
        state.fluxData.ll = f.ll ?? 0
        state.envParams[4].value = f.ll ?? 0
    })
    store.footerStats.fluxRxBytes += hex.length / 2
}