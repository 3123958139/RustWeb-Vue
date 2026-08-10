<!--
/**
 * @component TemplatePanel
 * @description 角色模板面板组件
 *
 * 本组件是一个演示性模板，展示如何为新角色创建面板。
 * 它不包含任何业务逻辑，仅演示角色注册表的使用方式。
 *
 * ## 设计目的
 * 1. 作为新角色面板的起点（复制此文件即可）
 * 2. 演示如何从角色注册表获取信息
 * 3. 展示权限检查和菜单生成
 *
 * ## 使用场景
 * 当需要添加新角色时：
 * 1. 在后端 `src/roles.rs` 的 ROLE_REGISTRY 中注册新角色（key、name、permissions）
 * 2. 运行 `npm run gen:api` 同步前端类型
 * 3. 在 `roles.ts` 的 MENU_CONFIG 中添加该角色的菜单
 * 4. 复制本文件为新面板
 * 5. 按需修改模板和脚本
 * 6. 在角色应用的路由中引入
 *
 * ## 依赖关系
 * - 角色注册表（roles.ts）：获取角色信息、权限、菜单
 * - 会话管理（session.ts）：获取当前用户信息
 * - 类型定义（types.ts）：TypeScript 类型支持
 *
 * 注意：
 * - 本组件不依赖任何应用内的 Store
 * - 不依赖应用内的导航栏或样式
 * - 复制后可独立使用
 -->
<template>
  <!-- 组件根容器 -->
  <div class="template-root">
    <!-- 工具栏：显示标题和说明 -->
    <div class="template-toolbar">
      <span class="template-title">模板面板</span>
      <span class="template-badge">角色模板示例</span>
      <div class="template-spacer"></div>
      <span class="template-clock">仅展示角色注册表驱动逻辑，无业务功能</span>
    </div>

    <!-- 说明面板：如何为新角色创建面板 -->
    <div class="template-panel">
      <div class="template-panel-header">如何为新角色创建面板</div>
      <div class="template-panel-body">
        <!-- 步骤列表 -->
        <ol class="template-steps">
          <li>
            在后端 <code>src/roles.rs</code> 的 <code>ROLE_REGISTRY</code> 中注册新角色
            （key、名称、权限），运行 <code>npm run gen:api</code> 同步前端类型，
            并在 <code>roles.ts</code> 的 <code>MENU_CONFIG</code> 中添加菜单，
            admin 端用户管理下拉框将自动出现该角色；
          </li>
          <li>复制本文件为新面板，按需展示业务内容；</li>
          <li>将 <code>isRegisteredRole("new_role")</code> 的结果作为路由 meta 守卫，
            该角色登录后即可访问本面板。</li>
        </ol>
        <div class="template-note">
          本模板位于公共包 <code>packages/shared/src/template/</code>，不依赖任何角色应用
          （无应用内 store / 导航栏 / 样式），复制后由角色应用的路由引入。
        </div>
      </div>
    </div>

    <!-- 信息网格：展示当前角色信息和权限 -->
    <div class="template-grid">
      <!-- 角色信息面板 -->
      <div class="template-panel">
        <div class="template-panel-header">当前角色</div>
        <div class="template-panel-body">
          <table class="template-table">
            <tbody>
              <tr>
                <td>角色 key</td>
                <!-- 使用可选链（?.）和空值合并（??）操作符 -->
                <td>{{ currentRole?.key ?? "未登录" }}</td>
              </tr>
              <tr>
                <td>角色名称</td>
                <td>{{ currentRole?.name ?? "—" }}</td>
              </tr>
              <tr>
                <td>注册表角色数</td>
                <td>{{ roles.length }}</td>
              </tr>
              <tr>
                <td>是否为已注册角色</td>
                <td>{{ isRegistered ? "是" : "否" }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- 权限演示面板 -->
      <div class="template-panel">
        <div class="template-panel-header">权限演示（权限点：fj200c_information:monitor）</div>
        <div class="template-panel-body">
          <!-- 遍历显示用户所有权限 -->
          <span v-for="permission in permissions" :key="permission" class="template-tag">
            {{ permission }}
          </span>
          <div class="template-hint">
            本面板要求 <code>fj200c_information:monitor</code> 权限，
            当前角色{{ hasMonitorPermission ? "拥有" : "不拥有" }}该权限。
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * 脚本部分
 *
 * 使用 Vue 3 Composition API（<script setup> 语法糖）
 *
 * 语法说明：
 * - `<script setup>`: 编译时语法糖，自动注册所有顶层变量/函数为模板可用
 * - `lang="ts"`: 启用 TypeScript 支持
 * - `computed()`: 创建计算属性（响应式，依赖变化时自动更新）
 * - `ref()`: 创建响应式引用（本文件未使用，但常见）
 */

import { computed } from "vue";
import { getAllRoles, findRole, getPermissionsByRole } from "../roles";
import { Permission } from "../types";
import { loadSession } from "../session";

// ============ 响应式数据 ============

/**
 * 所有已注册角色的列表
 *
 * 使用场景：
 * - 显示注册表角色总数
 * - 可扩展为角色选择器
 *
 * 注意：
 * - 只读取一次，不响应角色注册表变化
 * - 如果需要响应式，应使用 computed 或 ref
 */
const roles = getAllRoles();

/**
 * 当前登录用户
 *
 * 从 localStorage 会话中获取用户信息。
 * 注意：这不是响应式的，因为 localStorage 不会触发 Vue 的响应式系统。
 *
 * 获取逻辑：
 * 1. 调用 loadSession() 从 localStorage 加载会话
 * 2. 如果会话存在，返回 user 对象
 * 3. 如果会话不存在，返回 null
 */
const currentUser = loadSession()?.user ?? null;

// ============ 计算属性 ============

/**
 * 当前用户的角色定义
 *
 * 根据用户角色从注册表缓存（loadRoleRegistry 拉取自后端 /api/meta/roles）查找角色配置。
 *
 * 类型说明：
 * - 返回类型：`RoleDef | undefined`
 * - 使用 computed 创建响应式计算属性
 * - 当 currentUser 变化时自动重新计算
 */
const currentRole = computed(() => (currentUser ? findRole(currentUser.role) : undefined));

/**
 * 当前角色是否已注册
 *
 * 用于判断用户角色是否在注册表中定义。
 *
 * 使用场景：
 * - 路由守卫：验证角色是否有效
 * - UI 显示：提示用户角色是否有效
 */
const isRegistered = computed(() => !!currentRole.value);

/**
 * 当前用户的权限列表
 *
 * 根据用户角色从注册表获取权限点数组。
 *
 * 使用场景：
 * - 权限检查：判断用户是否有某个权限
 * - UI 显示：展示用户所有权限
 */
const permissions = computed(() =>
  currentUser ? getPermissionsByRole(currentUser.role) : []
);

/**
 * 是否拥有 fj200c_information:monitor 权限
 *
 * 演示权限检查的使用方式。
 *
 * 使用场景：
 * - 条件渲染：根据权限显示/隐藏元素
 * - 权限验证：检查用户是否有访问权限
 */
const hasMonitorPermission = computed(() =>
  permissions.value.includes(Permission.Fj200cInformationMonitor)
);
</script>

<style scoped>
.template-root {
  padding: 16px;
  font-family: inherit;
  color: #333;
}

.template-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.template-title {
  font-size: 18px;
  font-weight: 600;
}

.template-badge {
  font-size: 12px;
  background: #fdf6ec;
  color: #e6a23c;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid #f5dab1;
}

.template-spacer {
  flex: 1;
}

.template-clock {
  font-size: 13px;
  color: #999;
}

.template-panel {
  border: 1px solid #ebeef5;
  border-radius: 6px;
  margin-bottom: 16px;
  background: #fff;
}

.template-panel-header {
  padding: 10px 14px;
  border-bottom: 1px solid #ebeef5;
  font-weight: 600;
  font-size: 14px;
  background: #fafafa;
  border-radius: 6px 6px 0 0;
}

.template-panel-body {
  padding: 14px;
}

.template-steps {
  margin: 0;
  padding-left: 20px;
  line-height: 2;
}

.template-steps code,
.template-note code,
.template-hint code {
  background: #ecf5ff;
  color: #409eff;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.template-note {
  margin-top: 12px;
  font-size: 13px;
  color: #999;
}

.template-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  align-items: start;
}

@media (max-width: 1000px) {
  .template-grid {
    grid-template-columns: 1fr;
  }
}

.template-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.template-table td {
  padding: 6px 8px;
  border-bottom: 1px solid #f0f2f5;
}

.template-table td:first-child {
  color: #909399;
  width: 40%;
}

.template-tag {
  display: inline-block;
  margin: 2px 6px 2px 0;
  padding: 2px 8px;
  border-radius: 4px;
  background: #ecf5ff;
  color: #409eff;
  font-size: 12px;
}

.template-hint {
  margin-top: 12px;
  color: #909399;
  font-size: 13px;
}
</style>
