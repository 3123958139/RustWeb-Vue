# Demo-Test3 — UDP 组播通信监控系统

基于 Tauri v2 + Vue 3 + Vite 6 + TypeScript + Rust 的桌面应用，用于实时监控多路 UDP 组播数据流。

## 功能

- 8 路 UDP 连接实时接收与显示（4×4 卡片网格）
- 主/备双链路自动切换（飞控帧 IP11 ↔ IP15，1 秒超时）
- 流式帧解析：逐字节匹配 `EB 90 5B 01~04` 帧头，和校验验证
- 4 槽无锁共享帧缓冲区（`ArcSwap` + `AtomicU32` CAS 去重）
- 3 路串口并发发送（`QuadFrame` 数据 → 串口）
- INI 配置文件在线编辑
- 内置模拟数据源（mock feature）

## 命令

```powershell
npm run dev                          # Vite 开发服务器（端口 1420）
npm run build                        # 类型检查 + 构建
npm run tauri dev                    # 启动桌面应用
npm run tauri build                  # 生产构建
npm run tauri -- dev --features mock # 启动 + 模拟数据源
```

## 项目结构

```
src/                     # Vue 3 前端
  App.vue                # 主界面：4×4 卡片 + 状态栏 + 配置对话框
  main.ts                # 入口，挂载 Element Plus
src-tauri/
  src/
    lib.rs               # Tauri 命令注册、服务启停管理
    backend/process.rs   # UDP 接收线程、帧提取、主备切换、事件发射
    frontend/update.rs   # udp-data 事件发射器
    prelude.rs           # 公共重导出
    main.rs              # 桌面入口
    packages/dch/        # 内部 crate
      src/common/
        quad_frame.rs    # QuadFrame：4 槽无锁共享帧
        config.rs        # INI 配置单例
        frame_extractor  # 逐字节帧解析器
        trace.rs         # 日志追踪
        utils.rs         # 工具函数
      src/control/
        udp.rs           # UDP 组播控制器
        com.rs           # 串口创建与收发（跨平台 serialport 4.x）
        frame_extractor  # 流式帧提取
```

## 帧格式

| 偏移    | 长度 | 字段         | 说明                  |
|-------|----|------------|---------------------|
| 0-2   | 3  | `EB 90 5B` | 帧同步头                |
| 3     | 1  | `01~04`    | 槽位索引                |
| 4-7   | 4  | SEQ        | 时序号 (LE u32)        |
| 8-92  | 85 | 载荷         | 有效数据                |
| 93-94 | 2  | CHECKSUM   | 前 93 字节累加和 (LE u16) |

详见 `AGENTS.md`。

## 串口跨平台配置

3 路串口（轨迹/设备/遥测）通过 `config.ini` 配置，`serialport` 4.x 支持跨平台。

`PORTNAME` 需按平台填写：

| 平台      | 示例                             |
|---------|--------------------------------|
| Windows | `COM101`, `COM3`               |
| Linux   | `/dev/ttyS101`, `/dev/ttyUSB0` |
| macOS   | `/dev/cu.usbserial-*`          |

详见 `config.ini` 中 `[ComFTJ1CTrajectory]`、`[ComFTJ1CEquipment]`、`[ComFTJ1CTelemetering]` 节的配置说明。
