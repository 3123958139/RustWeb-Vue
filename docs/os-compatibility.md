# 操作系统兼容性分析与建议（Windows / Ubuntu）

> 结论先行：**官方推荐最低版本为 Windows 10 x64 与 Ubuntu 22.04+**。Win7 仅在打了 UCRT 补丁后"大概率能跑"但没有保证；Ubuntu 18.04 以上仅运行可、构建需 22.04+。本文所有结论均基于项目源码核实，非臆测。

---

## 一、Windows

### 1.1 项目代码层面的核实（无阻碍）

- `src/main.rs:309` 的关闭信号仅用 `tokio::signal::windows::ctrl_shutdown()`，属跨平台标准 API，Win7 即存在
- 全 `src/` 无 `winapi` crate、无 `std::os::windows` FFI 调用（`cfg(windows)` 仅上文一处）
- 硬件依赖均为 Win7 时代 API：`serialport 4`（CreateFile / GetCommState 串口）、`socket2 0.5`（SO_REUSEADDR UDP 组播）

→ **项目自己写的代码没有任何 Win7 用不了的系统调用。**

### 1.2 但二进制是否可运行由工具链决定（三个硬约束）

1. **Rust 工具链不锁版本**：项目无 `rust-toolchain.toml`，`deploy.bat` 直接用本机当前 `cargo build --release --features embedded`；`Cargo.lock` 已提交仓库锁定依赖版本。
   关键事实：**Rust 1.77（2024-03）起 `x86_64-pc-windows-msvc` 目标最低支持版本提升到 Windows 10**，Win7 降为 tier 3——"能编译、不测试、坏了不修"。新版本 stdlib 或依赖一旦调用 Win10 独有 API，exe 启动即崩且无人会修。
2. **MSVC 动态链接 Universal CRT**：VS Build Tools 编译的 exe 依赖 `ucrtbase.dll` 与 `api-ms-win-crt-*.dll`，**Win7 SP1 默认没有**，必须手工安装 KB2999226（Universal C Runtime）补丁，否则启动即报"缺少 api-ms-win-crt-runtime-l1-1-0.dll"。
   另：Visual Studio Build Tools 2022 本身只支持 Win10+，**在 Win7 上连编译环境都装不起来**。
3. **浏览器端约束**：`packages/shared/tsconfig.json` target 为 ES2020，前端构建产物面向现代浏览器；Win7 官方支持的浏览器最高为 Chrome/Edge 109（2023-01 停更），之后无安全更新。

### 1.3 Windows 结论

| 场景 | 结论 |
|---|---|
| 编译环境 | **最低 Windows 10 x64**（rustup / VS2022 Build Tools 均要求） |
| 运行环境 | **最低 Windows 10 x64（官方推荐）** |
| Win7 SP1（仅运行） | 项目代码无阻碍，但需手工打 KB2999226 补丁 + 现代工具链兼容性无保证，不推荐 |

> Win7 本身已于 2020-01 结束微软官方支持，即使能跑也不应作为生产部署目标。

---

## 二、Ubuntu

### 2.1 事实依据

- 后端为 x86_64 Linux 二进制，Rust 对 `x86_64-unknown-linux-gnu` 仅有 glibc 2.17+ 的静态要求；项目依赖全为纯 Rust（rustls/crypto）无 openssl 系统库依赖
- 模型：运行仅需 glibc → Ubuntu 18.04（glibc 2.27）即可运行
- 但构建链（rustup）新版本要求 glibc 2.35+（即 Ubuntu 22.04+）；前端构建需 Node.js 18+

### 2.2 Ubuntu 结论

| 场景 | 结论 |
|---|---|
| 仅运行后端二进制 | Ubuntu 18.04+（glibc 2.27+）|
| 完整构建（Rust + 前端） | **Ubuntu 22.04+（推荐）**，20.04 亦可运行旧版 rustup |

---

## 三、部署建议（正式对外声明口径）

1. 官方支持矩阵对外统一声明为：**Windows 10 x64 / Windows Server 2016+，Ubuntu 22.04+（构建）、18.04+（仅运行）**
2. Win7 场景如果客户确有存量机器：说明"未经验证、无保证"，并提示需打 KB2999226 与浏览器安全风险，建议客户洽谈升级
3. 依赖版本已由提交的 `Cargo.lock` 锁定，可复现构建（客户环境如需进一步固定可再锁 rust-toolchain.toml）
4. 如需将 Win7 纳入承诺范围：需用旧工具链（Rust ≤ 1.76）单独构建并做 Win7 真机回归测试——代价高、收益低，不推荐

---

## 附：核实来源

| 条目 | 来源 |
|---|---|
| 无 winapi / 仅 tokio 信号 | `src/main.rs:309-321`，grep 全 `src/` |
| MSVC 工具链要求 | `docs/tutorial/07-使用与维护手册.md:27` |
| 工具版本要求（Rust stable / Node 18+） | `docs/tutorial/07-使用与维护手册.md:9-16` |
| 前端 ES2020 | `packages/shared/tsconfig.json:3` |
| 无 rust-toolchain.toml | 根目录 glob 无此文件 |
| 单 exe 构建方式 | `deploy.bat`（`cargo build --release --features embedded`） |