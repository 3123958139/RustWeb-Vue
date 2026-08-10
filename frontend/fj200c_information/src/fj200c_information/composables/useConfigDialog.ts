/**
 * 配置编辑组合式函数
 *
 * 读取/保存服务端 config-fj200c_information.ini 配置文件。
 * Web 版不再读写本地文件，全部通过后端 API 操作。
 *
 * 返回值包含：
 * - visible / loading / saving / content：响应式状态
 * - open / close / save：操作方法
 */

import { ref } from "vue";
import { fj200c_informationApi } from "@/api";

/**
 * 配置编辑组合式函数
 *
 * @returns 配置编辑相关的状态和方法
 */
export function useConfigDialog() {
  /** 对话框是否可见 */
  const visible = ref(false);
  /** 加载状态（读取配置时） */
  const loading = ref(false);
  /** 保存状态 */
  const saving = ref(false);
  /** 配置文件内容（可编辑） */
  const content = ref("");

  /**
   * 打开配置对话框并加载文件内容
   * 调用 fj200c_informationApi.getConfig() 从后端读取配置文件
   */
  const open = async () => {
    visible.value = true;
    loading.value = true;
    try {
      const response = await fj200c_informationApi.getConfig();
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

  /**
   * 保存配置文件内容
   * @returns 操作结果（success / message）
   */
  const save = async () => {
    saving.value = true;
    try {
      const response = await fj200c_informationApi.saveConfig(content.value);
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
