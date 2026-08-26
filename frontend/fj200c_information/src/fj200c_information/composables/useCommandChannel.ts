/**
 * 命令通道组合式函数
 *
 * 一个命令通道包含：命令类型选择、数据输入、发送功能。
 * 可选 transform 函数将输入的命令数据转为十六进制字节串。
 * 发送时调用 fj200c_informationApi.sendCommand，通过后端 MPSC 通道发送到连接的设备。
 */

import {ref} from "vue";
import {fj200c_informationApi} from "@/api";

/** 命令通道配置 */
export interface ChannelConfig {
    /** 命令类型下拉选项 */
    cmdOptions: string[];
    /**
     * 可选的转换函数：将命令类型和原始输入转为十六进制字节串
     * @param type  - 用户选择的命令类型
     * @param input - 用户输入的原始数据
     * @returns 十六进制字节串
     */
    transform?: (cmdOptions: string[], type: string, input: string) => string;
}

/**
 * 命令通道组合式函数
 *
 * @param config - 通道配置（选项列表和转换函数）
 * @returns 命令通道的状态和方法
 */
export function useCommandChannel(config: ChannelConfig) {
    /** 当前选择的命令类型 */
    const cmdType = ref("");
    /** 当前输入的数据 */
    const cmdData = ref("");
    /** 命令发送日志（最新 100 条） */
    const cmdLog = ref<string[]>([]);

    /**
     * 发送命令
     *
     * 流程：
     * 1. 如果有 transform 函数，将类型和输入转为十六进制数据
     * 2. 调用 fj200c_informationApi.sendCommand 发送到后端
     * 3. 记录发送日志
     */
    async function sendCommand() {
        console.log(config.cmdOptions);
        // 有 transform 时转换数据，否则直接使用原始输入
        const data = config.transform
            ? config.transform(config.cmdOptions, cmdType.value, cmdData.value)
            : cmdData.value;
        if (!data) return; // 空数据不发送
        try {
            const response = await fj200c_informationApi.sendCommand(data);
            if (response.success) {
                cmdLog.value.unshift(`[${cmdType.value || "默认"}] ${data}`);
            } else {
                cmdLog.value.unshift(`发送失败: ${response.message || "未知错误"}`);
            }
            if (cmdLog.value.length > 100) cmdLog.value.length = 100;
        } catch (e) {
            console.error("发送命令失败:", e);
            cmdLog.value.unshift("发送异常，请检查服务状态");
            if (cmdLog.value.length > 100) cmdLog.value.length = 100;
        }
    }

    return {cmdType, cmdData, cmdLog, cmdOptions: config.cmdOptions, sendCommand};
}
