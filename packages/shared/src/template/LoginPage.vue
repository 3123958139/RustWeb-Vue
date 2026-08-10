<!--
  LoginPage - 通用登录页组件

  所有前端应用共享的登录模板，样式与 AppNavbar 统一（白底卡片 + Element Plus 主色蓝）。
  通过 props 支持模板复用：各应用只需传入自己的标题/副标题即可。

  用法：
  ```vue
  <template>
    <LoginPage title="登录" subtitle="欢迎回来" />
  </template>
  <script setup lang="ts">
  import { LoginPage } from "@shared";
  </script>
  ```

  props:
  - title      卡片标题（默认 "用户登录"）
  - subtitle   副标题（默认 "欢迎回来"）
  - footerText 底部提示文案（默认 "账号由管理员创建并分配角色"）
  - appKind    应用分类："user" | "admin"，决定登录成功后菜单来源（默认 "user"）
-->
<template>
  <div class="login-container">
    <!-- 动态 SVG 背景：宇航员太空漂浮 -->
    <div class="bg-scene" aria-hidden="true">
      <svg
        viewBox="0 0 1920 1080"
        preserveAspectRatio="xMidYMid slice"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <linearGradient id="space" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#04060f" />
            <stop offset="50%" stop-color="#0a1430" />
            <stop offset="100%" stop-color="#16284f" />
          </linearGradient>
          <radialGradient id="nebulaPurple" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0%" stop-color="rgba(124,80,220,0.3)" />
            <stop offset="100%" stop-color="rgba(124,80,220,0)" />
          </radialGradient>
          <radialGradient id="nebulaTeal" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0%" stop-color="rgba(40,180,200,0.2)" />
            <stop offset="100%" stop-color="rgba(40,180,200,0)" />
          </radialGradient>
          <radialGradient id="atmos" cx="0.5" cy="0.5" r="0.5">
            <stop offset="70%" stop-color="rgba(110,180,255,0.22)" />
            <stop offset="100%" stop-color="rgba(110,180,255,0)" />
          </radialGradient>
          <radialGradient id="earthGrad" cx="0.35" cy="0.3" r="0.9">
            <stop offset="0%" stop-color="#3d8fdc" />
            <stop offset="70%" stop-color="#1b5cad" />
            <stop offset="100%" stop-color="#0a2f6d" />
          </radialGradient>
          <radialGradient id="earthShade" cx="0.3" cy="0.3" r="0.95">
            <stop offset="0%" stop-color="rgba(255,255,255,0)" />
            <stop offset="60%" stop-color="rgba(10,20,50,0)" />
            <stop offset="100%" stop-color="rgba(4,10,28,0.55)" />
          </radialGradient>
          <radialGradient id="visorGrad" cx="0.4" cy="0.35" r="0.8">
            <stop offset="0%" stop-color="#7cc4ff" />
            <stop offset="60%" stop-color="#2f6fd6" />
            <stop offset="100%" stop-color="#0b1f4d" />
          </radialGradient>
          <radialGradient id="moonGlow" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0%" stop-color="rgba(255,250,220,0.25)" />
            <stop offset="100%" stop-color="rgba(255,250,220,0)" />
          </radialGradient>
        </defs>

        <!-- 深空背景 -->
        <rect width="1920" height="1080" fill="url(#space)" />

        <!-- 星云 -->
        <ellipse cx="1650" cy="220" rx="500" ry="320" fill="url(#nebulaPurple)" />
        <ellipse cx="220" cy="330" rx="460" ry="300" fill="url(#nebulaTeal)" />
        <ellipse cx="1500" cy="900" rx="420" ry="260" fill="url(#nebulaPurple)" opacity="0.5" />

        <!-- 星星（闪烁） -->
        <circle class="star" cx="120" cy="120" r="1.5" fill="#eaf4ff" />
        <circle class="star" cx="300" cy="200" r="2.2" fill="#eaf4ff" style="animation-delay: 0.4s" />
        <circle class="star" cx="520" cy="110" r="1.3" fill="#eaf4ff" style="animation-delay: 1.2s" />
        <circle class="star" cx="700" cy="180" r="1.8" fill="#eaf4ff" style="animation-delay: 0.8s" />
        <circle class="star" cx="880" cy="80" r="1.2" fill="#eaf4ff" style="animation-delay: 1.6s" />
        <circle class="star" cx="1050" cy="150" r="2" fill="#eaf4ff" style="animation-delay: 0.2s" />
        <circle class="star" cx="1230" cy="100" r="1.4" fill="#eaf4ff" style="animation-delay: 2s" />
        <circle class="star" cx="1380" cy="200" r="1.7" fill="#eaf4ff" style="animation-delay: 1s" />
        <circle class="star" cx="1580" cy="120" r="1.3" fill="#eaf4ff" style="animation-delay: 0.6s" />
        <circle class="star" cx="1720" cy="240" r="2.1" fill="#eaf4ff" style="animation-delay: 1.4s" />
        <circle class="star" cx="1850" cy="140" r="1.5" fill="#eaf4ff" style="animation-delay: 0.3s" />
        <circle class="star" cx="640" cy="300" r="1.1" fill="#eaf4ff" style="animation-delay: 2.2s" />
        <circle class="star" cx="1440" cy="320" r="1.4" fill="#eaf4ff" style="animation-delay: 1.8s" />
        <circle class="star" cx="90" cy="330" r="1.2" fill="#eaf4ff" style="animation-delay: 0.9s" />

        <!-- 十字星光 -->
        <g class="glint" transform="translate(620, 200)">
          <rect x="-7" y="-1.2" width="14" height="2.4" rx="1.2" fill="#dcebff" />
          <rect x="-1.2" y="-7" width="2.4" height="14" rx="1.2" fill="#dcebff" />
        </g>
        <g class="glint" transform="translate(1280, 300)" style="animation-delay: 1.5s">
          <rect x="-5" y="-0.9" width="10" height="1.8" rx="0.9" fill="#dcebff" />
          <rect x="-0.9" y="-5" width="1.8" height="10" rx="0.9" fill="#dcebff" />
        </g>
        <g class="glint" transform="translate(900, 130)" style="animation-delay: 3s">
          <rect x="-4" y="-0.7" width="8" height="1.4" rx="0.7" fill="#dcebff" />
          <rect x="-0.7" y="-4" width="1.4" height="8" rx="0.7" fill="#dcebff" />
        </g>

        <!-- 蓝色行星（地球） -->
        <g>
          <circle cx="240" cy="880" r="360" fill="url(#atmos)" />
          <circle cx="240" cy="880" r="300" fill="url(#earthGrad)" />
          <path
            d="M120 760 q60 -40 120 -10 q40 30 -10 70 q-60 30 -110 -10 Z"
            fill="rgba(110,210,150,0.4)"
          />
          <path
            d="M300 840 q80 -30 130 30 q20 60 -50 80 q-70 10 -100 -40 q-20 -40 20 -70 Z"
            fill="rgba(110,210,150,0.3)"
          />
          <path
            d="M180 980 q70 -20 110 30 q10 50 -50 60 q-60 10 -70 -40 q-10 -30 10 -50 Z"
            fill="rgba(110,210,150,0.28)"
          />
          <ellipse cx="150" cy="800" rx="90" ry="26" fill="rgba(255,255,255,0.22)" transform="rotate(-18 150 800)" />
          <ellipse cx="300" cy="960" rx="120" ry="30" fill="rgba(255,255,255,0.16)" transform="rotate(12 300 960)" />
          <ellipse cx="80" cy="990" rx="70" ry="20" fill="rgba(255,255,255,0.18)" transform="rotate(-8 80 990)" />
          <circle cx="240" cy="880" r="300" fill="url(#earthShade)" />
        </g>

        <!-- 宇航员（漂浮摆动） -->
        <g class="astro">
          <g transform="translate(1450, 400) scale(0.85) rotate(-10)">
            <!-- 背包 -->
            <rect x="-66" y="-52" width="46" height="118" rx="16" fill="#93a7c4" />
            <rect x="-58" y="58" width="10" height="10" rx="3" fill="#6f86a6" />
            <rect x="-44" y="60" width="10" height="10" rx="3" fill="#6f86a6" />
            <!-- 躯干 -->
            <rect x="-46" y="-40" width="88" height="104" rx="30" fill="#f2f6fc" />
            <rect x="-14" y="-10" width="26" height="30" rx="7" fill="#dbe6f2" />
            <rect x="-6" y="34" width="12" height="16" rx="5" fill="#8fa5c2" />
            <path d="M-46 -8 h88" stroke="#c9d6e6" stroke-width="2" fill="none" />
            <path d="M-46 20 h88" stroke="#c9d6e6" stroke-width="2" fill="none" />
            <!-- 胸口姓名 -->
            <text
              x="0"
              y="10"
              text-anchor="middle"
              font-family="Arial, sans-serif"
              font-size="13"
              font-weight="700"
              letter-spacing="1"
              fill="#2f6fd6"
            >dengch</text>
            <!-- 左臂（张开） -->
            <g transform="rotate(-42 -52 2)">
              <rect x="-92" y="-16" width="80" height="30" rx="15" fill="#f2f6fc" />
              <rect x="-98" y="-14" width="34" height="26" rx="13" fill="#dbe6f2" />
            </g>
            <!-- 右臂（张开） -->
            <g transform="rotate(38 52 2)">
              <rect x="12" y="-16" width="80" height="30" rx="15" fill="#f2f6fc" />
              <rect x="64" y="-14" width="34" height="26" rx="13" fill="#dbe6f2" />
            </g>
            <!-- 腿 -->
            <g transform="rotate(9 -22 66)">
              <rect x="-36" y="60" width="30" height="74" rx="15" fill="#f2f6fc" />
              <rect x="-40" y="126" width="40" height="18" rx="9" fill="#93a7c4" />
            </g>
            <g transform="rotate(-7 22 66)">
              <rect x="8" y="60" width="30" height="74" rx="15" fill="#f2f6fc" />
              <rect x="2" y="126" width="40" height="18" rx="9" fill="#93a7c4" />
            </g>
            <!-- 头盔与面罩 -->
            <circle cx="0" cy="-64" r="40" fill="#f2f6fc" />
            <circle cx="0" cy="-64" r="40" fill="none" stroke="#dbe6f2" stroke-width="3" />
            <circle cx="7" cy="-62" r="29" fill="url(#visorGrad)" />
            <ellipse cx="-2" cy="-72" rx="10" ry="6" fill="rgba(255,255,255,0.55)" transform="rotate(-24 -2 -72)" />
          </g>
        </g>

        <!-- 月亮 -->
        <g>
          <circle cx="1450" cy="640" r="64" fill="url(#moonGlow)" />
          <circle cx="1450" cy="640" r="46" fill="#e8e4d8" />
          <circle cx="1434" cy="626" r="8" fill="#d3cdbd" />
          <circle cx="1462" cy="648" r="5" fill="#d3cdbd" />
          <circle cx="1442" cy="658" r="4" fill="#d3cdbd" />
          <circle cx="1456" cy="622" r="3" fill="#d3cdbd" />
        </g>

        <!-- 漂浮物（缓慢自转） -->
        <g class="debris" transform="translate(760, 300)">
          <rect x="-7" y="-16" width="14" height="32" rx="7" fill="rgba(242,246,252,0.55)" />
        </g>
        <g class="debris" transform="translate(1280, 520)" style="animation-delay: -14s">
          <rect x="-16" y="-6" width="32" height="12" rx="6" fill="rgba(242,246,252,0.4)" />
          <rect x="-2" y="-14" width="4" height="28" rx="2" fill="rgba(242,246,252,0.35)" />
        </g>
        <g class="debris" transform="translate(1330, 250)" style="animation-delay: -26s">
          <circle r="8" fill="rgba(242,246,252,0.35)" />
        </g>
        <g class="debris" transform="translate(700, 620)" style="animation-delay: -33s">
          <rect x="-12" y="-4" width="24" height="8" rx="4" fill="rgba(242,246,252,0.4)" />
        </g>

        <!-- 远处小火箭 -->
        <g transform="translate(430, 150) rotate(-32)">
          <polygon points="-8,-34 8,-34 16,26 8,34 -8,34 -16,26" fill="#f2f6fc" />
          <polygon points="-16,26 16,26 8,44 -8,44" fill="#c9d6e6" />
          <circle cx="0" cy="-14" r="6" fill="url(#visorGrad)" />
          <ellipse class="flame" cx="0" cy="52" rx="5" ry="12" fill="#ffb347" />
          <ellipse class="flame" cx="0" cy="50" rx="2.5" ry="8" fill="#ffe08a" style="animation-delay: 0.15s" />
        </g>
      </svg>
    </div>
    <div class="login-wrapper">
      <el-card class="login-card" :body-style="{ padding: '32px' }">
        <template #header>
          <div class="card-header">
            <div class="logo-icon">
              <el-icon :size="34" color="#409eff"><Monitor /></el-icon>
            </div>
            <h2 class="login-title">{{ title }}</h2>
            <p class="login-subtitle">{{ subtitle }}</p>
          </div>
        </template>

        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          label-position="top"
          class="login-form"
          @submit.prevent="handleLogin"
        >
          <el-form-item label="邮箱" prop="email">
            <el-input
              v-model="form.email"
              type="email"
              placeholder="请输入邮箱"
              clearable
              size="large"
              :prefix-icon="Message"
            />
          </el-form-item>

          <el-form-item label="密码" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              placeholder="请输入密码"
              clearable
              size="large"
              :prefix-icon="Lock"
              @keyup.enter="handleLogin"
            />
          </el-form-item>

          <el-form-item>
            <el-button
              type="primary"
              :loading="loading"
              size="large"
              class="login-button"
              @click="handleLogin"
            >
              <el-icon v-if="!loading"><Right /></el-icon>
              {{ loading ? "登录中..." : "立即登录" }}
            </el-button>
          </el-form-item>

          <div class="form-footer">
            <span class="footer-text">{{ footerText }}</span>
          </div>
        </el-form>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { Lock, Message, Monitor, Right } from "@element-plus/icons-vue";
import { getAppAuthStore, getMenusByRole, getRoleAppUrl } from "..";
import type { FormInstance, FormRules } from "element-plus";

/**
 * 认证 Store 的最小接口形状
 *
 * 与 AppNavbar 相同，通过 getAppAuthStore() 解耦，
 * 不依赖任何具体应用的 Store 实现。
 */
interface AuthStoreShape {
  login: (email: string, password: string) => Promise<{ success: boolean; message?: string }>;
  isAuthenticated: boolean;
  userRole: string;
  logout: () => void;
}

const props = withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    footerText?: string;
    appKind?: "user" | "admin";
  }>(),
  {
    title: "用户登录",
    subtitle: "欢迎回来",
    footerText: "账号由管理员创建并分配角色",
    appKind: "user",
  },
);

const router = useRouter();
const authStore = getAppAuthStore<AuthStoreShape>();

const formRef = ref<FormInstance>();
const loading = ref(false);

const form = reactive({
  email: "",
  password: "",
});

const rules: FormRules = {
  email: [
    { required: true, message: "请输入邮箱", trigger: "blur" },
    { type: "email", message: "请输入正确的邮箱格式", trigger: "blur" },
  ],
  password: [
    { required: true, message: "请输入密码", trigger: "blur" },
    { min: 6, message: "密码长度不能少于6位", trigger: "blur" },
  ],
};

const handleLogin = async () => {
  if (!formRef.value) return;

  await formRef.value.validate(async (valid) => {
    if (!valid) return;

    if (!authStore) {
      ElMessage.error("认证服务未就绪");
      return;
    }

    loading.value = true;
    try {
      const result = (await authStore.login(form.email, form.password)) ?? {
        success: false,
        message: "登录失败",
      };

      if (result.success) {
        // 角色不属于本应用：跳转到该角色自己的应用
        if (!authStore.isAuthenticated) {
          const url = getRoleAppUrl(authStore.userRole, import.meta.env.DEV);
          if (url) {
            ElMessage.warning("该账号属于其他应用，正在跳转");
            window.location.href = url;
            return;
          }
          authStore.logout();
          ElMessage.error("该账号无法登录本应用");
          return;
        }

        ElMessage.success("登录成功");
        // 跳到当前角色菜单的第一个面板（注册表驱动）
        const menus = getMenusByRole(authStore.userRole, props.appKind);
        router.push(menus[0]?.children?.[0]?.path ?? menus[0]?.path ?? "/login");
      } else {
        ElMessage.error(result.message || "登录失败");
      }
    } catch {
      ElMessage.error("登录失败");
    } finally {
      loading.value = false;
    }
  });
};
</script>

<style scoped>
.login-container {
  position: fixed;
  inset: 0;
  overflow-y: auto;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 16px;
  background: linear-gradient(135deg, #1e3a5f 0%, #2a5298 55%, #1e3a5f 100%);
  box-shadow: inset 0 0 180px rgba(0, 0, 0, 0.25);
  /* 无论应用是否引入 Element Plus 暗色主题（dark css-vars），登录页固定使用浅色主题 */
  --el-bg-color: #ffffff;
  --el-bg-color-page: #f2f3f5;
  --el-bg-color-overlay: #ffffff;
  --el-fill-color: #f0f2f5;
  --el-fill-color-light: #f5f7fa;
  --el-fill-color-lighter: #fafafa;
  --el-fill-color-extra-light: #fafcff;
  --el-fill-color-dark: #ebedf0;
  --el-fill-color-darker: #e6e8eb;
  --el-fill-color-blank: #ffffff;
  --el-text-color-primary: #303133;
  --el-text-color-regular: #606266;
  --el-text-color-secondary: #909399;
  --el-text-color-placeholder: #a8abb2;
  --el-text-color-disabled: #c0c4cc;
  --el-border-color: #dcdfe6;
  --el-border-color-light: #e4e7ed;
  --el-border-color-lighter: #ebeef5;
  --el-border-color-extra-light: #f2f6fc;
  --el-border-color-dark: #d4d7de;
  --el-border-color-darker: #cdd0d6;
  --el-input-bg-color: #ffffff;
  --el-input-text-color: #303133;
  --el-input-placeholder-color: #a8abb2;
}

.bg-scene {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
  background: #04060f;
}

.bg-scene svg {
  display: block;
  width: 100%;
  height: 100%;
}

/* 星星闪烁 */
.star {
  animation: twinkle 3s ease-in-out infinite;
}

@keyframes twinkle {
  0%,
  100% {
    opacity: 0.15;
  }
  50% {
    opacity: 0.9;
  }
}

/* 十字星光闪烁 */
.glint {
  animation: twinkle 5s ease-in-out infinite;
}

/* 宇航员太空漂浮（上下浮动 + 轻微摇摆） */
.astro {
  animation: astroFloat 12s ease-in-out infinite;
  transform-box: fill-box;
  transform-origin: center;
}

@keyframes astroFloat {
  0%,
  100% {
    transform: translateY(0) rotate(3deg);
  }
  50% {
    transform: translateY(-26px) rotate(-4deg);
  }
}

/* 火箭尾焰闪烁 */
.flame {
  animation: flameFlicker 0.6s ease-in-out infinite;
  transform-box: fill-box;
  transform-origin: center;
}

@keyframes flameFlicker {
  0%,
  100% {
    opacity: 0.55;
    transform: scaleY(0.85);
  }
  50% {
    opacity: 1;
    transform: scaleY(1.15);
  }
}

/* 太空漂浮物缓慢自转 */
.debris {
  animation: debrisSpin 40s linear infinite;
  transform-box: fill-box;
  transform-origin: center;
}

@keyframes debrisSpin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .star,
  .glint,
  .astro,
  .flame,
  .debris {
    animation: none !important;
  }
}

.login-wrapper {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 440px;
}

.login-card {
  width: 100%;
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(64, 158, 255, 0.12);
  background: #ffffff !important;
  border: 1px solid #e4e7ed !important;
  animation: slideUp 0.6s ease-out;
}

.card-header {
  text-align: center;
  padding: 8px 0 4px;
}

.logo-icon {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 56px;
  height: 56px;
  border-radius: 14px;
  background: rgba(64, 158, 255, 0.1);
  border: 1px solid rgba(64, 158, 255, 0.25);
  margin: 0 auto 16px;
}

.login-title {
  color: #303133;
  margin: 0 0 8px 0;
  font-size: 28px;
  font-weight: 600;
  letter-spacing: 1px;
}

.login-subtitle {
  color: #909399;
  margin: 0;
  font-size: 14px;
  letter-spacing: 2px;
}

.login-form {
  margin-top: 16px;
}

.login-form :deep(.el-form-item__label) {
  color: #606266;
  font-weight: 500;
}

.login-form :deep(.el-input__wrapper) {
  /* !important：压过各应用全局主题（如 fj200c_main theme.css）对输入框的强制深色覆盖 */
  background: #ffffff !important;
  box-shadow: 0 0 0 1px #dcdfe6 inset !important;
  border-radius: 8px;
  transition: all 0.3s ease;
}

.login-form :deep(.el-input__wrapper:hover) {
  box-shadow: 0 0 0 1px #79bbff inset !important;
}

.login-form :deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 0 0 1px #409eff inset !important;
}

.login-form :deep(.el-input__inner) {
  color: #303133 !important;
}

.login-button {
  width: 100%;
  height: 48px;
  font-size: 16px;
  font-weight: 500;
  border-radius: 8px;
  background: linear-gradient(135deg, #409eff 0%, #337ecc 100%);
  border: none;
  color: #fff;
  transition: all 0.3s ease;
  letter-spacing: 2px;
}

.login-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(64, 158, 255, 0.35);
  background: linear-gradient(135deg, #79bbff 0%, #409eff 100%);
}

.login-button:active {
  transform: translateY(0);
}

.form-footer {
  text-align: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #f0f2f5;
}

.footer-text {
  color: #909399;
  font-size: 13px;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (max-width: 767px) {
  .login-container {
    padding: 8px;
  }

  .login-wrapper {
    max-width: 100%;
  }

  .login-card {
    border-radius: 12px;
  }

  .card-header {
    padding: 4px 0 0;
  }

  .login-title {
    font-size: 24px;
  }

  .login-subtitle {
    font-size: 13px;
  }

  .login-button {
    height: 44px;
    font-size: 15px;
  }
}

@media (min-width: 1024px) {
  .login-wrapper {
    max-width: 480px;
  }

  .login-title {
    font-size: 30px;
  }
}
</style>
