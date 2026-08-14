/**
 * @module useConfigDialog
 * @description 配置文件编辑组合式函数
 *
 * 读取/保存服务端 config-fj200c_main.ini 配置文件。
 * Web 版不再读写本地文件，全部通过后端 API 操作。
 */

import { ref } from "vue";
import { fj200cMainApi } from "@/api";

export function useConfigDialog() {
  /** 对话框是否可见 */
  const visible = ref(false);
  /** 加载状态（读取配置时） */
  const loading = ref(false);
  /** 保存状态 */
  const saving = ref(false);
  /** 配置文件内容（可编辑） */
  const content = ref("");

  /** 打开配置对话框并加载文件内容 */
  const open = async () => {
    visible.value = true;
    loading.value = true;
    try {
      const response = await fj200cMainApi.getConfig();
      if (response.success && response.data) {
        content.value = response.data.content;
      } else {
        content.value = "# 配置文件读取失败，服务启动后将在工作目录生成\n";
      }
    } catch {
      content.value = "# 配置文件读取失败\n";
    } finally {
      loading.value = false;
    }
  };

  /** 关闭配置对话框 */
  const close = () => {
    visible.value = false;
  };

  /** 保存配置文件内容 */
  const save = async () => {
    saving.value = true;
    try {
      const response = await fj200cMainApi.saveConfig(content.value);
      return response.success
        ? { success: true }
        : { success: false, message: response.message || "保存失败" };
    } catch (error: any) {
      return {
        success: false,
        message: error.response?.data?.message || "保存失败",
      };
    } finally {
      saving.value = false;
    }
  };

  return { visible, loading, saving, content, open, close, save };
}
