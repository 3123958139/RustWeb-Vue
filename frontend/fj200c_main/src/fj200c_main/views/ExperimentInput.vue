<!--
  ExperimentInput.vue — 试验信息录入页面（fj200c_main 模块）

  发动机试验前录入基本信息：发动机编号、电控器编号、传感器编号、试验项目等。
  数据通过 fj200cMainApi.saveExperiment() 持久化到后端 GlobalVar。

  适配说明（Tauri → Web）：
  - 原版通过 Tauri 窗口关闭时自动保存，Web 版改为"确定"按钮显式保存
  - 原版字段 snake_case（engine_no），Web 版改为 orval 生成的 camelCase（engineNo）
  - 不再使用独立子窗口，改为应用内路由页面
-->
<template>
  <ScaledPage>
    <div class="fj200c_main-root">
      <div class="fm-page input-page">
        <div class="fm-panel input-panel">
          <div class="fm-panel-header">试验信息录入</div>
          <div class="fm-panel-body">
            <el-form
              ref="formRef"
              :model="form"
              label-position="right"
              label-width="200px"
              class="experiment-form"
              v-loading="loading"
            >
              <el-form-item label="发动机编号" prop="engineNo">
                <el-input v-model="form.engineNo" placeholder="请输入发动机编号" />
              </el-form-item>
              <el-form-item label="燃气发生器编号" prop="gasGeneratorNo">
                <el-input v-model="form.gasGeneratorNo" placeholder="请输入燃气发生器编号" />
              </el-form-item>
              <el-form-item label="电控器编号" prop="controllerNo">
                <el-input v-model="form.controllerNo" placeholder="请输入电控器编号" />
              </el-form-item>
              <el-form-item label="转速传感器编号" prop="speedSensorNo">
                <el-input v-model="form.speedSensorNo" placeholder="请输入转速传感器编号" />
              </el-form-item>
              <el-form-item label="滑油温压一体传感器编号" prop="oilSensorNo">
                <el-input v-model="form.oilSensorNo" placeholder="请输入滑油温压一体传感器编号" />
              </el-form-item>
              <el-form-item label="试验项目" prop="testItem">
                <el-select v-model="form.testItem" placeholder="请选择试验项目" style="width: 100%">
                  <el-option label="检验试车" value="检验试车" />
                  <el-option label="匹配试车" value="匹配试车" />
                </el-select>
              </el-form-item>
              <el-form-item label="试验时间" prop="testTime">
                <el-input v-model="form.testTime" :disabled="true" />
              </el-form-item>
              <el-form-item>
                <el-button type="primary" @click="handleSubmit" :loading="submitting">确定</el-button>
              </el-form-item>
            </el-form>
          </div>
        </div>
      </div>
    </div>
  </ScaledPage>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { fj200cMainApi } from "@/api";
import type { ExperimentInfo } from "@shared/api/generated";
import ScaledPage from "@/fj200c_main/components/ScaledPage.vue";

const formRef = ref();
const submitting = ref(false);
const loading = ref(false);

/** 获取北京时间（UTC+8）并格式化为 YYYY-MM-DD HH:mm:ss */
function getBeijingTime(): string {
  const now = new Date();
  const utc = now.getTime() + now.getTimezoneOffset() * 60000;
  const bj = new Date(utc + 8 * 3600000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${bj.getFullYear()}-${pad(bj.getMonth() + 1)}-${pad(bj.getDate())} ${pad(bj.getHours())}:${pad(bj.getMinutes())}:${pad(bj.getSeconds())}`;
}

const form = reactive<ExperimentInfo>({
  engineNo: "",
  gasGeneratorNo: "",
  controllerNo: "",
  speedSensorNo: "",
  oilSensorNo: "",
  testItem: "检验试车",
  testTime: getBeijingTime(),
});

onMounted(async () => {
  form.testTime = getBeijingTime();
  loading.value = true;
  try {
    const response = await fj200cMainApi.getExperiment();
    if (response.success && response.data) {
      const saved = response.data;
      if (saved.engineNo) form.engineNo = saved.engineNo;
      if (saved.gasGeneratorNo) form.gasGeneratorNo = saved.gasGeneratorNo;
      if (saved.controllerNo) form.controllerNo = saved.controllerNo;
      if (saved.speedSensorNo) form.speedSensorNo = saved.speedSensorNo;
      if (saved.oilSensorNo) form.oilSensorNo = saved.oilSensorNo;
      if (saved.testItem) form.testItem = saved.testItem;
    }
  } catch {
    // 首次打开无保存数据，使用默认空值
  } finally {
    loading.value = false;
  }
});

async function handleSubmit() {
  submitting.value = true;
  try {
    const response = await fj200cMainApi.saveExperiment({ ...form });
    if (response.success) {
      ElMessage.success("保存成功");
    } else {
      ElMessage.error(response.message || "保存失败");
    }
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || "保存失败");
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
@import "@/fj200c_main/fj200c_main.css";

/* 信息录入页：整体在 1920×1080 舞台中水平垂直居中 */
.input-page {
  justify-content: center;
  align-items: center;
  overflow: hidden;
}

.input-panel {
  width: 640px;
  margin-bottom: 0;
}

.experiment-form {
  max-width: 640px;
}

.experiment-form :deep(.el-form-item__label) {
  color: var(--fm-text);
  font-size: 15px;
}
</style>
