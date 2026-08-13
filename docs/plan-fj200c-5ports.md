# fj200c 三路串口 → 5 路串口 改造计划

> 状态：已完成（2026-08-13）。最近一次提交 `e51623f` 已做一半迁移（后端 types/decode/abstract_com/handlers 与 openapi/generated 类型已 5 路），本次补齐剩余残留并修正迁移引入的 bug。

## 5 路串口定义（COM0–COM4）

| index | Section | 设备 | 帧格式 |
|---|---|---|---|
| 0 | `COM0` | ECU 电控器 | 42 字节 `EB 90 2A` 头 |
| 1 | `COM1` | Adam4015 环境采集 | `>` + 8 通道 ASCII |
| 2 | `COM2` | Adam4117 环境采集 | `>` + 8 通道 ASCII |
| 3 | `COM3` | Dyno 测功机 | Dyno 帧 |
| 4 | `COM4` | Flux 燃油流量 | `FF FF` 头 + u16 流量 |

## 后端变更

| 优先级 | 文件:行 | 内容 |
|---|---|---|
| 高 | `src/fj200c_main/fj200c_main/com.rs:191-220` | `init_all_from_config` 门控改 `count > 0/1/2/3/4`，idx 传 `0/1/2/3/4`（修复 adam4117/flux 与前者 idx 重复的 bug） |
| 高 | `com.rs:416` | `mock_configs` 追加 `("MOCK_COM3", 3)`、`("MOCK_COM4", 4)` |
| 高 | `com.rs:520-528, 544-551` | Dyno 事件 `connection_index: 2 → 3`，Flux 事件 `connection_index: 2 → 4` |
| 高 | `config-fj200c_main.ini:6,8-30` | `Count = 3 → 5`，追加 `[COM3]`、`[COM4]` 节 |
| 高 | `deploy.bat:113-156` | 默认 ini 模板 `Count = 5` + 补 `[COM3]`、`[COM4]` 节 |
| 中 | `src/fj200c_main/fj200c_main/mock.rs` | `MockProfile` 加 `Adam4117`/`Flux`；`from_section` 映射 5 个 MOCK_COM（MOCK_COM2→Adam4117、MOCK_COM3→Dyno、MOCK_COM4→Flux）；`generate_frame` 补 flux 帧（参考 `decode_flux` 18 字节 `FF FF` 头 + u16 流量） |
| 中 | `src/fj200c_main/fj200c_main/report.rs:78-88` | 区分 Adam4015/Adam4117 报表列（避免 `find()` 取到第一组） |
| 低 | 注释文案 | `handlers.rs`、`routes.rs:79/140`、`main.rs:48`、`embedded_assets.rs:35`、`api_docs.rs:72`、`src/fj200c_main/help_doc.md`（全文） |

## 前端变更（frontend/fj200c_main/）

| 优先级 | 文件:行 | 内容 |
|---|---|---|
| 高 | `src/fj200c_main/composables/useBackendPorts.ts:24` | 类型导入改 `Adam4015Fields, Adam4117Fields, DynoFields, EcuFields, FluxFields`（`AdamFields` 已删除，修复编译失败） |
| 高 | `useBackendPorts.ts:40-50` | switch 扩为 5 case：0 Ecu / 1 Adam4015 / 2 Adam4117 / 3 Dyno / 4 Flux |
| 高 | `useBackendPorts.ts:146-174` | 新增 `handleAdam4015`/`handleAdam4117`/`handleFlux`；确定两组 ADAM 通道与 envParams 映射 |
| 高 | `src/fj200c_main/store/dashboard.ts` | `FooterStats` 补 adam4015/adam4117/flux 统计；`envParams` 布局调整（含 Adam4117 通道 + Flux 流量）；新增 `adam4015Data`/`adam4117Data`/`fluxData` 状态 |
| 中 | `src/fj200c_main/components/StatusBar.vue` | 统计项加 Adam4015/Adam4117/Flux |
| 中 | `components/DashboardStats.vue` | 环境参数卡随 envParams 布局调整 |

## 无需改动

- `openapi/openapi.json`、`packages/shared/src/api/generated/model/`（e51623f 已 5 路，勿手改生成文件）
- `fj200c_information`（非三路模块）

## 验证

1. `cargo build` + `cargo test export_openapi`（DTO 变化后需 `npm run gen:api`）
2. `frontend/fj200c_main` 下 `npm run build`
3. ini 改 `Count=5` 后 `cargo run`：WS 事件 5 路齐全、模拟模式 DYNO/FLUX 有数据
