# 文件树整理计划：模板骨架一级目录 + 角色专有二级目录

## 目标

每个角色（前端、后端）统一文件树约定：

- **一级目录** = 从模板继承的骨架文件
- **二级目录**（角色名，如 `fj200c_information`）= 角色专有文件

## 一、后端 `src/`

约定：

- 一级目录 `src/<role>/`：`mod.rs` / `handlers.rs` / `routes.rs` / `service(s).rs`（模板四件套）
- 二级目录 `src/<role>/<role>/`：其余全部 rs 文件，内含 `mod.rs` 声明各子模块
- 一级 `mod.rs` 改为 `pub mod <role>;` + `pub use <role>::{...};` 再导出，保持既有
  `crate::<role>::x` 路径不变（api_docs.rs、routes.rs、跨模块引用零改动）
- 非 rs 文件留一级目录：`config-*.ini` 样本、`help_doc.md`（被 handlers.rs `include_str!` 相对引用，不可移）
- `admin` / `fw100` / `fw150` / `role_template` 只有模板四件套，无需二级目录

| 模块 | 移入二级目录的文件 |
|---|---|
| `fj200c_information` | `com.rs` `config.rs` `csv_writer.rs` `decode.rs` `frame_bundle.rs` `mock.rs` `mock_feeder.rs` `session.rs` `state.rs`（9 个） |
| `fj200c_main` | `abstract_com.rs` `com.rs` `config.rs` `decode.rs` `mock.rs` `report.rs` `state.rs` `types.rs`（8 个） |
| `ftj1c` | `com.rs` `config.rs` `models.rs` `process.rs` `quad_frame.rs` `state.rs` `udp.rs`（7 个） |
| `city3d` | `models.rs`（1 个） |

验证：`cargo check`。

## 二、前端 7 个应用（frontend/<role>/src/）

约定：

- 一级目录保留纯模板文件：`App.vue` `main.ts` `style.css` `vite-env.d.ts` `api/index.ts`
  `router/index.ts` `stores/auth.ts` `types/index.ts` `utils/responsive.ts` `views/Login.vue`
- 二级目录 `src/<role>/` 收编全部角色专有文件（views、api、components、composables、utils 等）
- 顺带删除 fj200c_information 的 Vite 脚手架残留 `components/HelloWorld.vue`、`assets/vue.svg`（未被引用）

| 应用 | 移入 `src/<role>/` |
|---|---|
| `fj200c_information` | 已有 components/composables/utils；再并入 `api/fj200c_information.ts`、`views/`（Monitor/Visual/Data/Config/Help） |
| `fj200c_main` | 已有 components/composables/store/styles；再并入 `api/fj200c_main.ts`、`views/`（7 个）、`types/vue-plugin-hiprint.d.ts` |
| `ftj1c` | 新建：`api/ftj1c.ts`、`views/Monitor.vue` |
| `fw100` | 新建：`api/fw100.ts`、`views/Panel.vue` |
| `fw150` | 新建：`api/fw150.ts`、`views/Panel.vue` |
| `city3d` | 新建：`api/city3d.ts`、`views/`、`components/`、`composables/`、`data/timeOfDay.ts`、`shaders/` |
| `admin` | 新建：`api/users.ts`、`views/`（Users/CreateUser/NoPermission） |

导入更新（约 40 处）：

- 各 `src/api/index.ts`：`./<role>` → `@/<role>/api/<role>`
- 各 `src/router/index.ts`：`@/views/<role>/X.vue` → `@/<role>/views/X.vue`
- city3d 内部交叉引用：`@/composables/`、`@/data/`、`@/shaders/`、`@/components/city3d/` → `@/city3d/...`
- fj200c 组件对 `@/api/fj200c_information` 的引用 → `@/fj200c_information/api/fj200c_information`

角色 api 文件只依赖 `@shared`，无相对导入；不产生循环依赖（现状即此模式）。

验证：7 个应用各自 `npm run build`（vue-tsc + vite build）。全程 `git mv` 保留历史。

## 三、文档

更新 `AGENTS.md`：后端模块结构章节、前端目录约定、"新增角色流程"补充二级目录与再导出说明。

## 风险

改动集中在 4 个后端 mod.rs + 前端导入路径，均为机械性变更；`cargo check` + 7×`npm run build` 全量验证。
