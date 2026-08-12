/**
 * 服务控制组合式函数
 *
 * 封装发动机监控服务的启动/停止/状态查询逻辑。
 * 默认每 3 秒轮询一次服务状态；传入 `isConnected` 后，
 * WebSocket 已连接时跳过轮询（事件流即实时状态），仅在断开时兜底。
 *
 * 组合式函数的职责：
 * - 封装相关的响应式状态
 * - 封装操作方法
 * - 在 onUnmounted 时清理副作用（定时器）
 * - 返回所有需要暴露的状态和方法
 */

import {onUnmounted, ref, type Ref} from "vue";
import {fj200c_informationApi} from "@/api";

/**
 * 服务控制组合式函数
 *
 * @param options - 可选：isConnected（WS 连接状态，连接时暂停轮询）
 * @returns 包含服务状态和操作方法的对象
 */
export function useService(_options: { isConnected?: Ref<boolean> } = {}) {
    /** 服务是否正在运行 */
    const serviceRunning = ref(false);
    /** 启动操作进行中 */
    const starting = ref(false);
    /** 停止操作进行中 */
    const stopping = ref(false);
    /** 错误信息 */
    const errorMessage = ref("");

    /** 状态轮询定时器 ID */
    let statusTimer: number | null = null;

    /**
     * 刷新服务状态
     * 调用后端 API 查询当前服务运行状态
     */
    const refreshStatus = async () => {
        try {
            const response = await fj200c_informationApi.getServiceStatus();
            if (response.success && response.data) {
                serviceRunning.value = response.data.running;
            }
        } catch (error: any) {
            errorMessage.value = error.response?.data?.message || "获取服务状态失败";
        }
    };

    /**
     * 启动服务
     * @returns 操作结果（success / message）
     */
    const startService = async () => {
        starting.value = true;
        errorMessage.value = "";
        try {
            const response = await fj200c_informationApi.startService();
            if (response.success) {
                serviceRunning.value = true;
                return {success: true};
            } else {
                errorMessage.value = response.message || "启动失败";
                return {success: false, message: errorMessage.value};
            }
        } catch (error: any) {
            errorMessage.value = error.response?.data?.message || "启动失败";
            return {success: false, message: errorMessage.value};
        } finally {
            starting.value = false;
        }
    };

    /**
     * 停止服务
     * @returns 操作结果（success / message）
     */
    const stopService = async () => {
        stopping.value = true;
        errorMessage.value = "";
        try {
            const response = await fj200c_informationApi.stopService();
            if (response.success) {
                serviceRunning.value = false;
                return {success: true};
            } else {
                errorMessage.value = response.message || "停止失败";
                return {success: false, message: errorMessage.value};
            }
        } catch (error: any) {
            errorMessage.value = error.response?.data?.message || "停止失败";
            return {success: false, message: errorMessage.value};
        } finally {
            stopping.value = false;
        }
    };

    /**
     * 切换服务状态（运行中则停止，停止则启动）
     * @returns 操作结果
     */
    const toggleService = async () => {
        if (serviceRunning.value) {
            return stopService();
        }
        return startService();
    };

    /** 轮询回调：WS 已连接时跳过 HTTP 轮询（事件流即实时状态，零请求） */
    const pollStatus = () => {
        // if (options.isConnected?.value) return;
        refreshStatus();
    };

    // 每 3 秒轮询一次状态（WS 断开时的兜底机制）
    statusTimer = window.setInterval(pollStatus, 3000);
    refreshStatus(); // 立即执行一次

    // 组件卸载时清除定时器
    onUnmounted(() => {
        if (statusTimer) clearInterval(statusTimer);
    });

    return {
        serviceRunning,
        starting,
        stopping,
        errorMessage,
        startService,
        stopService,
        toggleService,
        refreshStatus,
    };
}
