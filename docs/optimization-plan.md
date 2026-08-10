# RustWeb-Vue 优化计划

> 创建日期：2026-08-09
> 状态：按优先级分批执行，执行完成项打勾

## P0 安全（最高优先级）

- [ ] **jsonwebtoken 升级**：`Cargo.toml` 中 `jsonwebtoken = "9.2"` 存在 CVE-2026-25537（<10.3.0 受影响：畸形 nbf/exp 类型可绕过时间校验，CVSS 7.5），升级到 >=10.3.0 并核对 API 变化
- [ ] **JWT 密钥处理**：`src/common/jwt.rs:89,139` 每请求 `env::var("JWT_SECRET")` + 兜底硬编码 `"your-secret-key"`（部署忘设密钥即可伪造任意 token）；`exp/iat` 从 `usize` 改 `u64`
  - 启动时 `OnceLock` 缓存密钥与过期时间
  - 生产模式（`!cfg!(debug_assertions)`）缺失 `JWT_SECRET` 直接拒绝启动；dev 才用默认值
- [ ] **CORS 白名单化**：`src/main.rs:148-151` `allow_origin(Any)` → 新增 `CORS_ORIGINS` 环境变量白名单（逗号分隔），dev 模式才允许 `Any`
- [ ] **种子密码随机化**：`src/database.rs` 种子账号密码全为 `123456` → 首次插入时生成随机密码，打印到日志
- [ ] **登录速率限制**：bcrypt verify（~100ms CPU/次）无速率限制可被刷成 DoS → 内存限流（按 IP/邮箱，如 5 次/分钟，滑动窗口）

## P0 后端实时链路（多客户端/高帧率收益最大）

- [ ] **WS 预序列化广播**：`src/common/ws.rs:73` 每个客户端各自 `serde_json::to_string` 重复序列化；广播通道对事件 N 次深克隆
  - 生产端序列化一次，广播 `Arc<String>`（广播只克隆指针），ws_bridge 只转发
  - `src/common/ws.rs:73` `.unwrap()` 改为错误分支；`:80` Lagged warn 降为 trace/聚合
- [ ] **format_hex 查表**：`src/common/utils.rs:29` 每字节一次 `format!("{:02X}")` → 256 项静态查表，热点路径快 5-10 倍
- [ ] **decode 去重**：`src/fj200c_information/fj200c_information/session.rs` 同一帧 `decode()` 调用两次（CSV + Frame 事件）
- [ ] **Frame 事件节流**：115200 波特率下每帧全量推送 hex（通道频繁 Lagged）→ 200ms 节流（与 Payload/TableData 一致）
- [ ] **fj200c_main Arc 广播**：`src/fj200c_main/fj200c_main/com.rs:87-91` 每帧 `(*arc_fields).clone()` 深拷贝 → `Arc<ChannelData>` 事件
- [ ] **ftj1c 日志降噪**：`src/ftj1c/ftj1c/process.rs:918` 10ms 循环里 `info!`（每秒 ~100 条）→ `debug!`

## P0 前端体积与构建

- [ ] **Element Plus 按需导入**：7 个应用 `main.ts` 全量 `app.use(ElementPlus)` + 全量样式 → `unplugin-vue-components` + `ElementPlusResolver`（index chunk 1.08MB → ~250KB，7 应用共省 ~5-6MB）
- [ ] **echarts 按需引入**：`fj200c_information/Visual.vue:46`、`fj200c_main/ChartPanel.vue:13` 全量 `import * as echarts`（chunk 1107KB）→ `echarts/core` 按需注册
- [ ] **deploy.bat 并行构建**：7 个前端串行（2-4 分钟）→ `ForEach-Object -Parallel -ThrottleLimit 4`（顺序约束：后端编译必须等前端完成）
- [ ] **删除残留 node_modules**：`frontend/city3d/node_modules`、`frontend/fw150/node_modules`（幽灵依赖隐患）
- [ ] **轮询优化**：
  - `fj200c_information/src/fj200c_information/composables/useService.ts:110` 3s 无条件轮询 → 仅 WS 断开时轮询
  - `city3d`：`CityScene.vue:308` 与 `useCityData.ts:88` 双重 5s 事件轮询 → 单一定时器
- [ ] **payloadLog 数组优化**：`useFj200cInformationEvents.ts:84-87` 每帧 `slice(0,200)` 整体替换 → 原地截断 `length = 200`

## P1 运维与可诊断

- [x] **graceful shutdown**：`src/main.rs` → `with_graceful_shutdown`（Ctrl+C 时 CSV 不丢尾帧、WS 收关闭帧，Windows ctrl_shutdown + Unix SIGTERM）
- [x] **静态缓存头**：`src/main.rs` 7 个 ServeDir → hash 资源 `Cache-Control: public, max-age=31536000, immutable`，index.html 不缓存（`static_cache_headers` 中间件，dev/embedded 双模式生效）
- [x] **请求日志**：`tower-http` `trace` feature + `TraceLayer`（`new_for_http`）
- [x] **数据库启动优化**：`src/database.rs`
  - 约 90 条独立语句（各带隐式事务/fsync）→ 包进单事务
  - 3 条冗余索引（`idx_users_email`/`idx_users_username`/`idx_user_settings_user_id`，UNIQUE 约束已自动建索引）→ 删除
  - 无条件 8 次 bcrypt hash（阻塞 async 启动）→ 先查后算 + `spawn_blocking`
  - 连接池显式配置：`max_connections` + `busy_timeout`
- [x] **middleware 列裁剪**：每请求 `SELECT *` 全表（含密码哈希列）→ 只查必要列（`get_user_by_id`/`list_users` 不再取 `password_hash`，`create_user` 存在性检查改 `EXISTS` 标量；User 加 `#[sqlx(default)]`）
- [x] **unwrap 清理**：锁毒恢复（全部 `lock/read/write().unwrap()` → `unwrap_or_else(|e| e.into_inner())`）、HTTP 路径 panic 改错误返回（`fj200c_main/service.rs:39` `shared_port_data` → `ok_or` 错误返回，`:153` 模拟启动同样改防御）
- [x] **依赖清理**：`dotenv` → `dotenvy`、`tokio` 裁剪 features（macros/net/sync/signal/rt-multi-thread）、`rust-embed`/`mime_guess` 改 optional（仅 embedded 启用）、`tower-http` 加 `trace` feature（Cargo.toml 已全部落实）

## P2 工程化

- [x] **vite 共享工厂**：抽 `build/vite.base.ts` 工厂（base/port/alias/dedupe/Element Plus 按需/代理/manualChunks 分包/chunkSizeWarningLimit）+ 7 个应用改为 3 行调用（`appDir` 显式传入，因工厂内 `__dirname` 实测指向 `build/`）；7 应用构建通过，element-plus/echarts/vue-vendor/vendor 分包生效
- [x] **重复代码抽 @shared**：`utils/responsive.ts` 收敛为 `@shared/responsive.ts`（6 应用本地文件改 re-export，保持调用点不变；旧"Volar 限制"说明已过时——@shared 早就在提供组件）；路由守卫抽 `@shared/router.ts` 的 `createAppRouter` 工厂（7 应用统一守卫，差异参数化：homePath / noPermission "menu"|"403"；`useAuthStore` 传 `() => any` 因 StoreDefinition 泛型擦除，与 registerAuthStoreGetter 同款）
- [x] **CSV 写入移出采集线程**：`fj200c_information` 新增 `csv_sink.rs`（mpsc 通道 + 独立 std::thread），`session.rs` 采集线程只做非阻塞 send，文件创建/写行/flush/Shutdown 落盘全部在写入线程；`shutdown()` flush 尾帧并 join 保证不丢数据
- [ ] **fw100/fw150 参数化合并**（架构级，**已评估，本次不执行**）：后端 fw100 93 行 / fw150 113 行（仅差 Fw150LedgerItem schema 类型）；前端仅差 api facade（生成 tag）与 Panel.vue（CSS 前缀/列宽/文案）。收益有限（省 ~100 行重复），但需同步改 2 端口/base/workspaces + deploy.bat + main.rs 托管 + embedded_assets + roles + api_docs + routes 共约 8 处配置，风险中等。结论：保留现状，如未来新增同类台账角色再考虑参数化。

## 已确认无需优化

- 串口/UDP IO 与帧解析均在专用 std::thread，未阻塞 tokio async（spawn_blocking 非必需）
- 前端路由全部懒加载；依赖 workspaces 单实例
- 帧提取器与四槽缓冲（CAS + ArcSwap）无锁设计良好
- city3d 分页已有上限（最大 200）
- fw100/fw150 为内存演示数据（无 SQL）

## 执行顺序

1. [x] P0 安全（依赖升级 → jwt → CORS → 种子 → 限流）
2. [x] P0 后端实时链路（ws.rs → utils → session/com/process）
3. [x] `cargo build` + `cargo test export_openapi` 验证
4. [x] P0 前端（依赖 → main.ts → 组件 → deploy.bat）
5. [x] P1 运维（graceful shutdown / 缓存头 / 请求日志 / 数据库 / 列裁剪 / unwrap / 依赖清理）
6. [x] P2 工程化（vite 工厂 / @shared 收敛 / CSV 写入线程 / fw100+fw150 已评估不合并）
7. [x] 7 个前端 `npm run build` + 后端 `cargo build --release --features embedded` 全量验证

## 执行结果

- **P0/P1 全部完成**；P2 完成 3 项，fw100/fw150 合并经评估后决定不执行（结论见上）
- 前端：7 应用共享 `build/vite.base.ts` 工厂（Element Plus 按需、echarts 按需、manualChunks 分包）；`responsive.ts` 与 `createAppRouter` 路由守卫收敛到 `@shared`
- 后端：P1 全部完成（含 graceful shutdown、缓存头、TraceLayer 早已就位）；CSV 磁盘 IO 移出采集线程（`csv_sink.rs`）
- 全量验证通过：`cargo test export_openapi`（OpenAPI 防漂移）+ `cargo build --release --features embedded` + 7 个前端构建（残余 11 条历史 warning，无 error）
