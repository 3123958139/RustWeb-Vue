<!--
  配置文件编辑页

  由导航栏"打开配置"进入，替代原 App.vue 的 ConfigDialog 弹框。
  直接展示 config-fj200c_information.ini 内容，保存通过后端 /fj200c_information/config 接口。
-->
<template>
  <div class="fj200c_information-root">
    <div class="fj200c_information-page">
      <div class="fj200c_information-toolbar">
        <span class="toolbar-title">打开配置</span>
        <el-tag size="small" type="info">config-fj200c_information.ini</el-tag>
        <div class="spacer"></div>
        <ServiceNavButton />
        <el-button size="small" @click="router.back()">返回</el-button>
        <el-button type="primary" size="small" :loading="saving" @click="onSave">保存</el-button>
      </div>

      <div class="fj200c_information-panel">
        <div class="fj200c_information-panel-header">配置文件内容</div>
        <div class="fj200c_information-panel-body">
          <div v-if="loading" class="config-empty">加载中…</div>
          <!--
            el-input type="textarea"：多行文本输入框
            :rows="24" 设置初始显示行数
            spellcheck="false" 禁用拼写检查（配置文件不需要）
          -->
          <el-input
            v-else
            v-model="content"
            type="textarea"
            :rows="24"
            spellcheck="false"
            class="fj200c_information-config-editor"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import ServiceNavButton from "@/fj200c_information/components/ServiceNavButton.vue";import { useConfigDialog } from "@/fj200c_information/composables/useConfigDialog";

const router = useRouter();
/**
 * 使用配置编辑组合式函数
 * 解构获取：loading（加载状态）、saving（保存状态）、content（文件内容）、
 * open（加载配置）、save（保存配置）
 */
const { loading, saving, content, open, save } = useConfigDialog();

/** 组件挂载时自动加载配置文件内容 */
onMounted(open);

/** 保存配置文件 */
const onSave = async () => {
  const result = await save();
  if (result.success) {
    ElMessage.success("配置已保存");
  } else {
    ElMessage.error(result.message || "保存失败");
  }
};
</script>

<style scoped>
@import "@/fj200c_information/fj200c_information.css";

.config-empty {
  color: var(--fj200c_information-text-secondary);
  padding: 12px 0;
}
</style>
