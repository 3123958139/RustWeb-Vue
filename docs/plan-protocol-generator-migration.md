# 迁移计划：RustWeb-Vue 新增 `protocol_generator` 角色

## 背景与差异分析

demo-protocol（Tauri v2 桌面应用）的业务逻辑 = 协议表格编辑器 + CSV 参数表编辑器 + 4 种导出（JSON/Markdown/Excel/打印）。

Tauri 专属能力与 Web 架构的替代方案：

| Tauri 能力 | Web 替代 |
|---|---|
| `save_protocol`/`load_protocol`（原生对话框）| 前端 Blob `<a download>` 下载 / `<input type=file>` 上传解析 |
| `export_excel`（对话框）| 后端生成 xlsx 二进制 → 前端 Blob 下载 |
| `export_markdown` | 后端返回文本 → 前端弹窗/下载 |
| `load/save_default_csv`（app_data_dir）| 后端服务器 `parameters.csv`（UTF-8 BOM + 种子内容）|
| `load/save_csv`（任意路径）| 后端 `csv/parse` + `csv/serialize` 端点 |
| `open_csv_editor` 多窗口 + `csv-updated` 事件 | 同应用第二个路由页 `/protocol_generator/csv` |
| 打印 hiprint | 原样迁移 `vue-plugin-hiprint` |

## A. 后端（`src/`）

1. `src/common/models.rs`：`Permission` 枚举加 `ProtocolGeneratorMonitor`
2. `src/roles.rs`：`ROLE_REGISTRY` 加 `RoleDef { key: "protocol_generator", name: "protocol_generator", permissions: &[ProtocolGeneratorMonitor] }`
3. 新模块 `src/protocol_generator/`（两级目录约定）：
   - 一级骨架：`mod.rs` / `handlers.rs` / `routes.rs` / `services.rs`
   - 二级 `src/protocol_generator/protocol_generator/`：`models.rs`（`ProtocolField` / `ParameterDef`）+ `generator.rs`（Excel/Markdown 生成、默认 CSV 读写、种子内容）
   - handler（tag="protocol_generator"）：
     - `GET  /api/protocol_generator/default-csv` → `Vec<ParameterDef>`
     - `PUT  /api/protocol_generator/default-csv` ← `Vec<ParameterDef>`
     - `POST /api/protocol_generator/markdown` ← `{title, data}` → `{content}`
     - `POST /api/protocol_generator/excel` ← `{title, data}` → xlsx 二进制流
     - `POST /api/protocol_generator/csv/parse` ← `{content}` → `Vec<ParameterDef>`
     - `POST /api/protocol_generator/csv/serialize` ← `Vec<ParameterDef>` → `{content}`（带 BOM）
   - `routes.rs` 用 `permission_middleware(Permission::ProtocolGeneratorMonitor, ...)` 保护
4. `src/routes.rs`：`.nest("/api/protocol_generator", ...)`
5. `src/api_docs.rs`：注册 handler + schemas；断言更新（paths 42→47，operations 53→59）
6. `src/database.rs`：种子账号 `protocol_generator`（uuid `...000000000008`）
7. `src/main.rs`：`mod protocol_generator;` + dev 静态托管 `dist-protocol_generator`
8. `src/embedded_assets.rs`：`ProtocolGeneratorAssets` + 三条路由
9. `Cargo.toml`：加 `rust_xlsxwriter = "0.97.0"`（csv 已有）

## B. 共享包

10. `packages/shared/src/roles.ts`：`MENU_CONFIG`（协议编辑 `/protocol_generator/editor`、CSV 参数表 `/protocol_generator/csv`）+ `ROLE_APP_URLS`（dev 5180 / prod `/protocol_generator`）
11. `npm run gen:api` 重新生成 `generated/`

## C. 新前端 `frontend/protocol_generator`（复制 fw100 骨架）

12. 复制 fw100 → 改 `package.json` / `vite.config.ts`（app=protocol_generator, port=5180, 无 ws）
13. 角色专属 `src/protocol_generator/`：
    - `types/protocol.ts`（ProtocolField / CSharpTypes / getTypeSize / CsvParameter）
    - `utils/protocol.ts`（recalcFields）
    - `api/protocol-generator.ts`（facade + Blob 下载/上传辅助）
    - `components/ProtocolEditor.vue`、`components/CsvEditor.vue`
    - `views/Editor.vue` + `views/Csv.vue`
14. 骨架适配：router / stores/auth.ts / api/index.ts / types/index.ts / main.ts（hiprint 插件）
15. 根 `package.json` workspaces + 根目录 `npm install`（vue-plugin-hiprint）

## D. 部署与验证

16. `build-frontends.ps1`（8 个应用）、`deploy.bat`（提示 echo）
17. `AGENTS.md` 同步更新
18. 验证：`cargo test export_openapi` → `npm run gen:api` → `cargo check` → 新前端 `npm run build` → 手工冒烟