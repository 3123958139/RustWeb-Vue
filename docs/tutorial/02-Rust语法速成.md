# 02 Rust 语法速成（以本项目代码为教材）

> 适用对象：Rust 零基础或入门不久的新手。
> 教学目标：不是系统学习 Rust，而是**看懂并修改本项目的 Rust 代码**——语法点全部用项目真实代码举例，每个例子都标注源码位置，建议边读边打开文件对照。
> 全文约 2 万字。如果你有编程经验（Java/Python/JS 均可），预计 4~6 小时可读完并消化。
> 本文所有源码位置均已按当前仓库状态核对（8 个前端、8 个角色、两级目录模块结构、五路串口、`Arc<str>` 预序列化广播等）。

---

## 2.1 先建立正确心态：Rust 的"三座大山"其实没那么可怕

新手听说 Rust 难，主要难在三个概念：**所有权（Ownership）**、**借用（Borrowing）**、**生命周期（Lifetime）**。本项目的代码大量使用这些概念，但你不需要精通它们才能开始——你只需要掌握**阅读模式**和**修改模式**：

| 场景 | 你需要会什么 |
|---|---|
| 读代码 | 能看出"这个变量被谁拥有""这个引用是从哪来的" |
| 小改代码 | 能照着周围代码的样子抄（编译器会告诉你哪里错了） |
| 大改代码 | 才需要真正理解所有权规则 |

Rust 编译器的报错信息是全球公认最好的——它会告诉你具体改法。所以**大胆编译**，让编译器当你的老师。本项目后端改动后用 `cargo build` 验证，报错看不懂就问，或者贴给 AI。

还有一个好消息：本项目代码风格统一、注释详尽（几乎每个语法点都有中文注释），是最好的 Rust 阅读材料之一。

---

## 2.2 工具链准备（5 分钟）

```powershell
# 1. 安装 rustup（Windows 用官方安装器，或 winget install Rustlang.Rustup）
# 2. 验证
rustc --version      # 编译器
cargo --version      # 构建/包管理工具（npm 的 Rust 版）
rustup --version     # 工具链管理
# 3. VS Code 装 rust-analyzer 扩展（自动分析、悬停提示、跳转定义）
# 4. 进入项目根目录
cargo check          # 只检查不编译产物，比 cargo build 快（日常开发用这个）
cargo run            # 编译并运行（启动 Axum 后端 :3000）
cargo build --release --features embedded   # 生产构建（单 exe，内嵌 8 个前端 dist）
```

> 术语：**crate** = 包（一个 Cargo 项目）；**module** = 模块（一个 .rs 文件或目录）；`cargo` 类似 `npm`，但 Rust 是编译型语言，"npm run" 在 Rust 里是 `cargo run`。
> 注意：前端构建要在各自的 `frontend/*` 子目录执行 `npm run build`；类型同步走根目录的 `npm run gen:api`（详见 2.16 节）。

---

## 2.3 变量、类型与 let

### 2.3.1 基本变量

```rust
// src/config.rs —— 环境变量读取（最典型的 let 用法）
pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")          // 读环境变量（返回 Result<String>）
        .unwrap_or_else(|_| "3000".to_string())    // 出错时用默认值 "3000"
        .parse()?;                                 // 字符串转 u16（? 见 2.5 节）
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://rustweb.db".to_string());
    Ok(AppConfig { port, database_url })
}
```

新手要掌握的点：

1. **`let` 声明变量，默认不可变**：`let x = 5;` 之后 `x = 6` 会编译报错。要可变需写 `let mut x = 5;`。本项目大量利用这一点——不可变变量让代码意图清晰。
2. **类型可以推断**：`let port: u16 = ...` 显式标注了类型；很多地方可以省略 `let x = 5`（自动推断 i32）。
3. **`.to_string()`**：任何可打印类型转 `String`。
4. **`.unwrap_or_else(闭包)`**：`Result`/`Option` 出错时的兜底值模式——**项目里"配置有默认值"全部用这个模式**。
5. **`.parse::<T>()`**：字符串解析为数值类型，返回 `Result`（可能失败）。

### 2.3.2 常量与静态变量

```rust
// src/fj200c_information/fj200c_information/state.rs —— 模块常量（二级目录）
pub const CONFIG_PATH: &str = "config-fj200c_information.ini";  // 编译期常量

// src/common/ws.rs —— 全局静态广播发送端（event_broadcast! 宏生成的形态）
// 实际上各角色模块里直接调宏生成，见 src/fj200c_information/mod.rs：
// crate::event_broadcast!(FJ200C_TX, fj200c_information_tx);
// 展开后大致是：
static FJ200C_TX: OnceLock<broadcast::Sender<crate::common::ws::EventPayload>> = OnceLock::new();
```

| 关键字 | 含义 | 区别 |
|---|---|---|
| `const` | 编译期常量 | 无内存地址，内联展开；类型必须是字面量可表示的 |
| `static` | 全局静态变量 | 有固定内存地址，生命周期为整个程序 |
| `static mut` | 全局可变静态 | **禁止直接使用**（不安全），本项目用 `OnceLock`/`AtomicBool`/`RwLock` 包一层 | 

Rust 不允许随意访问可变全局变量（数据竞争），所以本项目所有全局状态都用 `OnceLock`（单例容器）、`AtomicBool`（原子布尔）、`RwLock`（读写锁）、`ArcSwap`（无锁热替换）包裹——**看到这些类型，就知道这是"全局状态"**。

### 2.3.3 元组与结构体字段访问

```rust
// src/common/models.rs —— 结构体字段访问
let email = &user.email;          // 取字段（借用）
let name = user.username.clone(); // 取字段（克隆，拥有所有权）
```

```rust
// src/common/auth/handlers.rs —— 登录 handler 的返回值是元组形态的组合
// (State(db), Json(login_data)) 是 Axum 提取器的组合，本质是元组；
// Rust 里 let (a, b) = (1, 2); 就是元组解构赋值
```

---

## 2.4 结构体与枚举（本项目最核心的两种类型）

### 2.4.1 结构体（struct）

```rust
// src/common/models.rs —— 用户模型（节选）
#[derive(Debug, Serialize, Deserialize, FromRow, Clone, utoipa::ToSchema)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    #[sqlx(default)]
    pub password_hash: String,    // 序列化时跳过！永不下发到前端
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

要点：

1. `pub` = 公开字段，模块外可访问（本项目几乎所有字段都是 pub，简化跨模块访问）。
2. `#[derive(...)]` 自动实现 trait（见 2.8 节宏）：`Clone`（可复制）、`Serialize/Deserialize`（JSON 转换）、`FromRow`（sqlx 从数据库行映射）、`ToSchema`（OpenAPI 文档）。
3. `#[serde(skip_serializing)]` 属性：JSON 输出时**排除**该字段——密码哈希永不外泄。`#[sqlx(default)]`：查询未 SELECT 该列时映射为默认值（中间件/列表接口常用）。
4. **创建结构体**：`User { id, username, ... }` 字段名与变量名相同时可简写（`User { id, ... }` 中的 `id` 即 `id: id`）。

结构体的方法定义（impl 块）：

```rust
// src/common/models.rs —— User 的方法
impl User {
    /// 计算用户的权限列表（查角色注册表）
    pub fn permissions(&self) -> Vec<Permission> {
        roles::permissions_for(&self.role)
    }

    /// 判断是否拥有某权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}
```

要点：`&self` = 只读借用（类似 `this` 但不可修改）；`&mut self` = 可变借用；`self` = 消费所有权。本项目方法几乎全是 `&self`（只读查询）。

### 2.4.2 枚举（enum）——本项目最常用的类型

```rust
// src/common/models.rs —— 权限枚举（12 个变体）
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub enum Permission {
    Fj200cInformationMonitor,      // fj200c_information 面板：发动机监控
    Fw100Monitor,                  // fw100 面板：设备台账
    Ftj1cMonitor,                  // ftj1c 面板：UDP 组播通信监控
    Ftj1cHelp,                     // ftj1c 帮助页（唯一需要此权限的端点）
    City3dView,                    // city3d 面板：城市 3D 展示
    Fw150Monitor,                  // fw150 面板：设备台账
    Fj200cMainMonitor,             // fj200c_main 面板：发动机测控
    ProtocolGeneratorMonitor,      // protocol_generator 面板：通信协议生成器
    UsersRead,                     // 用户读取（admin）
    UsersWrite,                    // 用户写入（admin）
    UsersDelete,                   // 用户删除（admin）
    SystemAdmin,                   // 系统管理标志（admin）
}
```

要点：

1. 枚举成员默认没有值（unit variant），用于"标记"场景——权限就是纯标记。
2. `PartialEq, Eq` 用于 `==` 比较（`user.has_permission` 里 `contains(permission)` 就依赖它）；`Hash` 用于放 HashSet 键。
3. serde 对枚举默认序列化为**字符串**（如 `"UsersRead"`），前端 orval 生成同名 TS 枚举——这就是前后端权限点类型同步的基础。
4. 角色注册表（`src/roles.rs` 的 `ROLE_REGISTRY`）是权限→角色的唯一事实来源，前端运行时从 `GET /api/meta/roles` 拉取。

枚举也可以**携带数据**——这是项目里事件系统的基础：

```rust
// src/fj200c_information/mod.rs —— 事件枚举（WS 推送的载荷类型）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]   // 判别标签 + 变体名转 snake_case
pub enum Fj200cInformationEvent {
    /// 帧事件：连接索引、原始十六进制、帧类型、28 个解码字段
    Frame {
        connection_index: usize,
        hex: String,
        frame_type: String,
        fields: Vec<String>,
    },
    /// 原始数据事件（200ms 节流推送）
    Payload {
        connection_index: usize,
        hex: String,
    },
    /// 表格数据事件（SharedData 字段行）
    TableData {
        connection_index: usize,
        rows: Vec<TableRow>,
    },
}
```

这里的 `#[serde(tag = "type", rename_all = "snake_case")]` 叫**内部标签枚举（internally tagged enum）**，序列化结果形如：

```json
{ "type": "frame", "connection_index": 0, "hex": "EB903C...", "frame_type": "试验数据下载", "fields": [...] }
```

前端 JS 用 `switch (event.type)` 分发——这就是前后端事件协议的契约。

另一个带数据枚举的典型是 fj200c_main 的**五路通道数据**：

```rust
// src/fj200c_main/fj200c_main/types.rs —— 五路串口数据的统一枚举
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ChannelData {
    Ecu(EcuFields),              // COM0 电控器：28 字段（含 FaultCodeFlags 27 个布尔故障位）
    Adam4015(Adam4015Fields),    // COM1 环境采集：channels: [f64; 8]
    Adam4117(Adam4117Fields),    // COM2 环境采集：channels: [f64; 8]
    Dyno(DynoFields),            // COM3 测功机：njzs 扭矩转速 / nj 扭矩 / njgl 扭矩功率
    Flux(FluxFields),            // COM4 燃油流量：ll
}
```

匹配方式：

```rust
let data: &ChannelData = ...;
match data {
    ChannelData::Ecu(f) => println!("Ng 转速: {}", f.ng_speed),
    ChannelData::Adam4015(f) | ChannelData::Adam4117(f) => println!("通道0: {}", f.channels[0]),
    ChannelData::Dyno(f) => println!("扭矩: {}", f.nj),
    ChannelData::Flux(f) => println!("流量: {}", f.ll),
}
```

### 2.4.3 枚举是"类型安全的 if"

在别的语言里，你可能会写 `if (type == 1)` 或魔法字符串；Rust 用枚举 + match 保证编译器强制处理所有分支（见 2.6 节）。新增一种设备 = 给 `ChannelData` 加一个变体，编译器会列出所有需要更新的 match——**改一处漏一处的 bug 从根上消失**。

---

## 2.5 Option 与 Result：没有 null 的世界

### 2.5.1 Option：可能有值

```rust
// src/common/ws.rs —— WS 桥的可选初始消息（连接建立时立即推送的快照）
pub async fn ws_bridge_with_initial(
    tx: broadcast::Sender<EventPayload>,
    socket: WebSocket,
    log_prefix: &str,
    initial_text: Option<String>,   // 要么 Some(内容)，要么 None（无）
) {
    // ...
    if let Some(text) = initial_text {
        if sender.send(Message::Text(text)).await.is_err() {
            return; // 客户端已断开
        }
    }
    // ...
}
```

| 模式 | 说明 | 本项目例子 |
|---|---|---|
| `Option<T>` | 有值/无值 | WS 初始快照、配置可选字段、`config::global()` 返回的 `Option<&Config>` |
| `.unwrap()` | 有值直接取出，无值 panic（崩溃） | **避免在生产路径用**，测试里常见 |
| `.unwrap_or(default)` | 无值用默认值 | 配置读取 |
| `.ok_or(err)?` | 无值转成错误并返回 | 见下 |
| `if let Some(x) = ...` | 有值才处理 | 见 2.6 |

### 2.5.2 Result：可能出错

```rust
// src/common/jwt.rs —— 验证 token（Result 的教科书用法）
pub fn verify_token(token: &str) -> Result<Uuid, jsonwebtoken::errors::Error> {
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret().as_bytes()),  // secret() 来自启动时缓存的 OnceLock
        &validation,
    )?;   // decode 返回 Result，? 直接传播错误
    Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken))
}
```

`Result<T, E>` 的含义：**成功时是 T，失败时是 E**。项目 HTTP 层统一 `Result<T, AppError>`，工具层各用各的错误类型（如 jwt 用库自带错误）。

新手必会的 Result 操作：

```rust
let x: Result<u32, String> = Ok(42);

x.unwrap()                // 42；失败则 panic（程序崩溃）
x.expect("说明文字")       // 同上，带自定义错误信息（比 unwrap 好）
x?                        // 成功取出 42；失败则函数提前返回错误（★最常用）
x.map(|v| v * 2)          // 成功时转换值：Ok(84)
x.map_err(|e| ...)        // 失败时转换错误类型
x.ok()?                   // 只关心值，错误直接抛（转 Option）
x.is_ok() / x.is_err()    // 判断
```

### 2.5.3 项目里最经典的组合：链式调用（登录 + 限流）

```rust
// src/common/auth/handlers.rs —— 登录 handler（含滑动窗口限流）
login_data.validate()?;    // 校验失败 → 400

// 登录限流：按 IP:email 滑动窗口 60 秒 / 5 次（src/common/rate_limit.rs）
let limiter_key = format!("{}:{}", ip, login_data.email);
if let Err(wait) = rate_limit::check_and_record(&limiter_key) {
    return Err(AppError::too_many_requests(format!("尝试过于频繁，请 {} 秒后重试", wait)));
}

let user = AuthService::login(&db, login_data)
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;   // 登录失败 → 400
rate_limit::clear(&limiter_key);                           // 登录成功清除失败记录
let token = jwt::create_token(&user)?;                     // 签发失败 → 500
Ok(Json(ApiResponse::success(LoginResponse { token, user })))
```

**读法口诀**：看到 `?` 就念"出错了就返回"，看到 `.map_err(|e| AppError::xxx(...))` 就念"把错误转换成 HTTP 错误码"。

---

## 2.6 模式匹配：match / if let / let else

### 2.6.1 match：穷尽所有分支

```rust
// src/fj200c_information/fj200c_information/decode.rs —— 帧类型字节 → 枚举（match 的经典用法）
pub fn frame_type_from_byte(b: u8) -> FrameType {
    match b {
        0xEF => FrameType::CSSZZL,        // 参数设置
        0xED => FrameType::CSDQZL,        // 参数读取
        0xDE => FrameType::SYSJXZZL,      // 试验数据下载（实时遥测）
        0xDC => FrameType::SYSJSK,        // 试验数据首块（开始 CSV 记录）
        0xBD => FrameType::SYSJZJK,       // 试验数据中间块（写入 CSV）
        0xDB => FrameType::SYSJMK,        // 试验数据末块
        0xBF => FrameType::JBCSQCZL,      // 基本参数清除
        0xBE => FrameType::SYSJQCZL,      // 试验数据清除
        _ => FrameType::NULL,             // 通配分支：其他值都到这里
    }
}
```

要点：

1. **必须穷尽**：match 要求覆盖所有情况，否则编译错误。`_` 是"其他所有"。
2. 每个分支是表达式，match 整体可以赋值：`let s = match x { 1 => "a", _ => "b" };`。
3. 模式可以是字面量、枚举成员、带 `|` 的多模式（`MockProfile::Adam | MockProfile::Adam4117 => ...`，见 `src/fj200c_main/fj200c_main/mock.rs`）、带守卫（`n if n > 10 => ...`）。

### 2.6.2 if let：只关心一种情况

```rust
// src/common/ws.rs —— WS 桥循环里的消息判断
match msg {
    Some(Ok(Message::Close(_))) | None => break,
    Some(Ok(_)) => continue,  // 忽略客户端文本/二进制消息
    Some(Err(_)) => break,
}
```

if let 简化"只关心一种情况"的 match：

```rust
if let Some(user) = request.extensions().get::<User>() {
    // 有用户（中间件已注入），处理业务
}
```

### 2.6.3 let-else：不满足就提前返回

`let ... else` 是"反直觉守卫"：模式匹配**失败**时执行 else 分支（通常提前 return）。它让"校验后继续"的代码不产生嵌套地狱：

```rust
let Some(token) = headers.get("token") else {
    return Err(StatusCode::UNAUTHORIZED);
};
// 到这里 token 一定可用
```

> 注意：else 分支必须"发散"（return / break / continue / panic）。本项目当前未大量使用 let-else（历史代码用 `ok_or` + `?` 表达同一意图），但读懂它即可。

### 2.6.4 模式匹配在项目中的使用场景清单

| 场景 | 位置 |
|---|---|
| 十六进制帧类型字节 → 枚举 | `fj200c_information/fj200c_information/decode.rs` |
| 五路串口数据 → 各字段结构 | `fj200c_main/fj200c_main/types.rs` 的 `ChannelData` |
| 模拟器按端口生成不同帧 | `fj200c_main/fj200c_main/mock.rs` 的 `MockProfile` |
| WS 消息类型分发 | `common/ws.rs`（`tokio::select!` 内） |
| 帧类型分支（CSV 状态机） | `fj200c_information/fj200c_information/session.rs` |
| 事件枚举分发（前端） | `useFj200cInformationEvents.ts`（TS 侧） |

---

## 2.7 trait：接口与抽象

### 2.7.1 trait 是什么

trait 类似 Java 的接口 / TypeScript 的 interface：定义一组方法签名，让不同类型实现同一套行为。

```rust
// src/common/io.rs —— 硬件抽象（本项目最重要的 trait，公共层）
/// 串口与模拟器的统一抽象：上层代码不关心数据来自真实硬件还是模拟器
pub trait IoControl: Send + Sync {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>>;
    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>>;
    fn set_timeout(&self, _timeout_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())   // 默认实现：不支持的实现可忽略
    }
}
```

实现者（两种）：

```rust
// src/fj200c_information/fj200c_information/com.rs —— 真实串口实现
pub struct ComControl { port: Mutex<Box<dyn SerialPort>> }
impl IoControl for ComControl {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(self.port.lock().unwrap().write(buf)?)
    }
    // ...
}

// src/fj200c_information/fj200c_information/mock.rs —— 模拟器实现
pub struct MockControl { /* 模拟数据生成状态 */ }
impl IoControl for MockControl {
    fn send(&self, _buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> { Ok(0) }
    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        // 生成一帧 20Hz 正弦+噪声模拟数据，写入 buf
        Ok(frame.len())
    }
    // ...
}
```

**这就是 Mock 开关的魔法来源**：`config-fj200c_information.ini` 里 `[Mock] InProcess = true` 时，启动服务就传 `MockControl`；否则传 `ComControl`。上层 `session.rs` 拿到的是 `Arc<dyn IoControl>`（trait 对象），完全不感知差异：

```rust
// src/fj200c_information/service.rs（实际代码节选）
let control: Arc<dyn IoControl> = if mock_enabled {
    Arc::new(MockControl::new())        // 模拟器
} else {
    Arc::new(ComControl::open(&port_name, baud, ...)?)   // 真实串口
};
```

fj200c_main 更进一步抽象了一层：`AbstractCom`（`src/fj200c_main/fj200c_main/abstract_com.rs`）持有 `Arc<dyn IoControl>` + `ComSpec`（帧头/数据区长度/尾部长度）+ 停止标志，五路串口共用同一套读取循环，差异全部收敛在 `ComSpec` 的五个构造函数里：

```rust
pub struct ComSpec {
    pub section: String,        // 配置节名（COM0..COM4）
    pub conn_idx: usize,        // 端口序号
    pub frame_header: Vec<u8>,  // 帧头
    pub frame_data_len: usize,  // 数据区长度
    pub frame_tail_len: usize,  // 尾部长度
}
// ECU:   ［0xEB,0x90,0x2A］ + 38B + 1
// Adam4015/4117: b'>' + 57
// Dyno/Flux: ［0xFF,0xFF］ + 14 + 2
```

### 2.7.2 trait 对象与泛型

| 写法 | 含义 | 本项目例子 |
|---|---|---|
| `Arc<dyn IoControl>` / `Box<dyn IoControl>` | trait 对象：运行时多态（堆上，动态分发） | 会话线程持有的 IO |
| `impl IoControl`（参数位置） | 泛型糖：编译期静态分发 | 较少 |
| `<T: Trait>` | 泛型约束 | `FrameExtractor` 内部、`serve_embedded<A: RustEmbed>` |
| `A: RustEmbed` | 泛型约束（嵌入式资源） | `embedded_assets.rs` |

### 2.7.3 本项目用到的标准库 trait（背下来）

| trait | 作用 | 看到它就想到 |
|---|---|---|
| `Serialize` / `Deserialize` | JSON 序列化/反序列化 | DTO 结构体必带 |
| `Clone` | 深度复制 | 跨线程传数据需要 |
| `Debug` | `{:?}` 打印调试 | 日志/测试 |
| `PartialEq` / `Eq` | `==` 比较 | 枚举、配置比较 |
| `Send` / `Sync` | 跨线程安全标记 | **所有跨线程的类型必须满足**，编译器强制检查 |
| `ToSchema` | OpenAPI 文档生成 | DTO 必带（utoipa） |
| `From<T>` / `Into<T>` | 类型转换 | `AppError` 自动转换（见 2.9） |
| `Default` | `Default::default()` 默认值 | 配置结构体、`EcuFields::default()` |
| `IntoResponse` | 转 HTTP 响应 | `AppError`、handler 返回值 |
| `FromRow` | 数据库行转结构体 | sqlx 查询结果 |
| `RustEmbed` | 编译期内嵌静态资源 | `embedded_assets.rs` 宏生成的结构体 |

---

## 2.8 宏：derive 宏、属性宏与声明式宏（含项目实际宏）

### 2.8.1 derive 宏：自动实现

`#[derive(Serialize)]` 不是装饰器，而是**代码生成器**：编译器展开后为你的结构体自动生成几十行 `Serialize` 实现代码。本项目里：

```rust
// src/city3d/city3d/models.rs —— Building 模型（一行顶几十行）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Building { ... }
```

新手只需要：**新写 DTO 时照抄现有 DTO 的 derive 行**（要什么能力就抄什么：要进 OpenAPI 就加 ToSchema；要 JSON 就加 Serialize/Deserialize；要跨线程传就加 Clone）。

### 2.8.2 属性宏：utoipa::path

```rust
// src/common/auth/handlers.rs —— 登录接口
#[utoipa::path(
    post,
    tag = "auth",
    path = "/api/auth/login",
    operation_id = "authLogin",
    request_body = LoginRequest,
    responses((status = 200, description = "登录成功", body = ApiResponse<LoginResponse>))
)]
pub async fn login(...) -> Result<Json<ApiResponse<LoginResponse>>, AppError> { ... }
```

这行注解生成 OpenAPI 文档条目（06 章详述）。**写新接口时必须加**，否则 `cargo test export_openapi` 的防漂移断言（48 个 path / 60 个操作）会失败。

### 2.8.3 声明式宏：macro_rules!（本项目公共宏）

本项目把重复的"全局单例"样板收敛成三个公共宏（架构优化 commit 61a4b8e），**理解这三个宏就理解了项目的全局状态布局**：

**① `config_singleton!` —— 全局只读配置单例（`src/common/config.rs`）**

```rust
#[macro_export]
macro_rules! config_singleton {
    ($global_ident:ident, $getter:ident, $setter:ident) => {
        static $global_ident: std::sync::OnceLock<crate::common::config::Config> =
            std::sync::OnceLock::new();
        pub fn $getter() -> Option<&'static crate::common::config::Config> {
            $global_ident.get()
        }
        pub fn $setter(cfg: crate::common::config::Config) -> Result<(), crate::common::config::Config> {
            $global_ident.set(cfg)
        }
    };
}
```

使用（一行生成"static + 两个函数"）：

```rust
// src/fj200c_information/fj200c_information/config.rs —— 整个文件就这几行
pub use crate::common::config::Config;
crate::config_singleton!(GLOBAL, global, set_global);
// 之后调用：config::global() → Option<&Config>，config::set_global(cfg)
```

> fj200c_main 因配置需要热替换，保持自实现 `OnceLock<RwLock<Option<Config>>>` + `clear_global()`（`src/fj200c_main/fj200c_main/config.rs`）——这就是"宏是收敛公共形态、特例自己写"的活例子。

**② `event_broadcast!` —— 全局广播通道单例（`src/common/ws.rs`）**

```rust
#[macro_export]
macro_rules! event_broadcast {
    ($tx_ident:ident, $getter:ident) => {
        static $tx_ident: std::sync::OnceLock<
            tokio::sync::broadcast::Sender<crate::common::ws::EventPayload>,
        > = std::sync::OnceLock::new();
        pub fn $getter() -> tokio::sync::broadcast::Sender<crate::common::ws::EventPayload> {
            $tx_ident
                .get_or_init(|| tokio::sync::broadcast::channel(1024).0)
                .clone()
        }
    };
}
```

使用：

```rust
// src/fj200c_main/mod.rs
crate::event_broadcast!(FJ200C_MAIN_TX, fj200c_main_tx);
// src/fj200c_information/mod.rs
crate::event_broadcast!(FJ200C_TX, fj200c_information_tx);
// src/ftj1c/mod.rs
crate::event_broadcast!(FTJ1C_TX, ftj1c_tx);
```

**③ `embed_assets!` / `embed_app_routes!` —— 8 个前端内嵌结构体与路由（`src/embedded_assets.rs`）**

```rust
// 生成 8 个 #[derive(RustEmbed)] 结构体，消灭手写 24 条路由
embed_assets!(
    AdminAssets => "frontend/admin/dist/",
    Fj200cInformationAssets => "frontend/fj200c_information/dist/",
    Fj200cMainAssets => "frontend/fj200c_main/dist/",
    Fw100Assets => "frontend/fw100/dist/",
    Fw150Assets => "frontend/fw150/dist/",
    Ftj1cAssets => "frontend/ftj1c/dist/",
    City3dAssets => "frontend/city3d/dist/",
    ProtocolGeneratorAssets => "frontend/protocol_generator/dist/",
);

// 生成每个应用的三条路由（/x、/x/、/x/*path），SPA 深链接回退 index.html
embed_app_routes!(
    Router::new(),
    "admin" => AdminAssets,
    "fj200c_information" => Fj200cInformationAssets,
    // ... 其余 6 个
)
```

新手不需要会写宏，但要**能读懂宏的调用**，并知道"想改这个行为要动哪里"——比如新增前端应用时，在 `embed_assets!` 里追加一行、在 `embed_app_routes!` 里追加一行。

---

## 2.9 错误处理：AppError 体系

### 2.9.1 统一错误类型

```rust
// src/common/error.rs
pub struct AppError {
    pub status_code: u16,   // HTTP 状态码
    pub message: String,    // 人类可读信息
}

impl AppError {
    pub fn bad_request(msg: String) -> Self { Self { status_code: 400, message: msg } }
    pub fn unauthorized(msg: String) -> Self { Self { status_code: 401, message: msg } }
    pub fn forbidden(msg: String) -> Self { Self { status_code: 403, message: msg } }
    pub fn not_found(msg: String) -> Self { Self { status_code: 404, message: msg } }
    pub fn too_many_requests(msg: String) -> Self { Self { status_code: 429, message: msg } }
    pub fn internal(msg: String) -> Self { Self { status_code: 500, message: msg } }
}
```

### 2.9.2 自动转 HTTP 响应

```rust
// src/common/error.rs —— 让 AppError 变成合法的 handler 返回值
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "success": false,
            "message": self.message,
        });
        (StatusCode::from_u16(self.status_code).unwrap(), Json(body)).into_response()
    }
}
```

因为实现了 `IntoResponse`，handler 返回 `Result<T, AppError>` 时，`Err(AppError)` 会被自动序列化成上面的 JSON 响应。

### 2.9.3 From 转换：错误自动升级

```rust
// src/common/error.rs —— 常见错误 → AppError 的自动转换
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::not_found("记录不存在".to_string()),
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() =>
                AppError::bad_request("记录已存在".to_string()),
            other => AppError::internal(other.to_string()),
        }
    }
}
impl From<jsonwebtoken::Error> for AppError { ... }   // → 401
impl From<bcrypt::Error> for AppError { ... }         // → 500
impl From<validator::ValidationErrors> for AppError { ... }  // → 400
```

**这个机制是 `?` 能工作的关键**：函数返回 `Result<_, AppError>` 时，如果内部错误类型实现了 `From<E> for AppError`，`?` 会自动调用转换。所以代码里可以随便写：

```rust
let user = sqlx::query_as::<_, User>("SELECT ...")
    .fetch_one(&db)
    .await?;    // sqlx::Error 自动转 AppError，一行都不用写转换
```

### 2.9.4 新手错误处理口诀

1. handler 返回 `Result<Json<ApiResponse<T>>, AppError>`。
2. 业务错误用工厂方法：`AppError::bad_request(...)`。
3. 内部错误靠 `From` + `?` 自动转。
4. 前端永远收到 `{ success, message, data? }`。

---

## 2.10 async/await 与 tokio

### 2.10.1 什么是异步

**同步代码**：一个请求占一个线程，线程阻塞等 IO（数据库、网络）。**异步代码**：少数线程轮流处理大量请求，阻塞等待期间让出线程。Axum 的 handler 全部是异步函数。

```rust
// src/main.rs —— 后端启动流程（332 行，按注释编号读）
#[tokio::main]        // 宏：把 main 变成 tokio 异步运行时入口
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();                                    // 1. 加载 .env
    tracing_subscriber::registry()                             // 2. 日志
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let config = AppConfig::load()?;                           // 3. 读取配置
    let pool = init_database(&config.database_url).await?;     // 4. 数据库（异步）
    crate::common::jwt::init()?;                               // 4.5 JWT 密钥缓存（OnceLock）
    let cors = { /* dev: Any；生产: CORS_ORIGINS 白名单，缺失拒绝启动 */ };
    let app = create_router(pool).layer(cors)
        .route("/", get(|| async { Redirect::temporary("/admin") }));
    // #[cfg(feature = "embedded")] → merge(embedded_assets::embedded_router())
    // #[cfg(not(feature = "embedded"))] → nest_service 8 个 dist-* 目录
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())             // Ctrl+C 优雅退出
        .await?;
    Ok(())
}
```

要点：
1. `async fn` 的函数调用**不会立刻执行**，返回一个 Future，必须 `.await` 才会推进。
2. `.await` 遇到阻塞会自动让出线程，其他请求乘机执行。
3. 项目中 handler 都是 `async fn`；HTTP/WS/数据库操作都有异步版本。
4. **CPU 密集任务不能用 async 处理**：bcrypt 哈希会阻塞线程。项目用 `tokio::task::spawn_blocking` 把它丢到阻塞线程池（种子密码生成、登录校验都是）：

```rust
// src/database.rs —— 种子密码 bcrypt（CPU 密集 → spawn_blocking）
let hash = tokio::task::spawn_blocking(move || {
    bcrypt::hash(password_clone.as_bytes(), bcrypt::DEFAULT_COST)
})
.await??;
```

### 2.10.2 tokio::select!：同时监听多个源

```rust
// src/common/ws.rs —— WS 桥的核心循环（所有角色共用一个）
loop {
    tokio::select! {
        msg = receiver.next() => {          // 分支一：客户端发来消息
            match msg {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => continue,    // 忽略客户端文本/二进制消息
                Some(Err(_)) => break,
            }
        }
        event = rx.recv() => {              // 分支二：广播通道有事件
            match event {
                Ok(payload) => {
                    // 载荷是预序列化的 Arc<str>，只克隆指针转成 String 帧
                    if sender.send(Message::Text(payload.to_string())).await.is_err() {
                        break;  // 客户端断开
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    trace!("客户端接收过慢，丢弃 {} 个滞后事件", n);  // 高频帧下属正常
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    }
}
```

`tokio::select!` 同时等待两个 future，**谁先完成执行谁**——这是"WS 双向通信"的惯用模式。`Lagged` 是 broadcast 通道的特性：接收端跟不上时旧消息被丢弃，用 `continue` 跳过即可（不让客户端拖慢系统）。

### 2.10.3 新手 async 避坑

1. **不要在 async 里做阻塞操作**（串口读、文件大读写、CPU 计算）——用 `spawn_blocking` 或 `std::thread`。
2. **`std::thread::sleep` 会阻塞整个 tokio 线程**——异步代码里用 `tokio::time::sleep`。
3. 本项目采集线程用 `std::thread`（阻塞串口读），HTTP 用 tokio，两者靠 broadcast 桥接——**不要试图把串口读改成 async**。

---

## 2.11 线程与并发（本项目并发全景）

### 2.11.1 ServiceRuntime：线程句柄管理（src/common/service.rs）

```rust
/// 服务运行时：线程句柄集合 + 停止进行中标志
pub struct ServiceRuntime {
    handles: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>>,  // 线程句柄（惰性初始化）
    stopping: AtomicBool,                                    // 停止进行中标志
}

impl ServiceRuntime {
    pub const fn new() -> Self { /* ... */ }
    pub fn push(&self, handle: thread::JoinHandle<()>) {     // 登记线程
        self.handles().lock().unwrap_or_else(|e| e.into_inner()).push(handle);
    }
    pub fn drain(&self) -> Vec<thread::JoinHandle<()>> {     // 取出全部句柄（停止时 join）
        self.handles().lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect()
    }
    pub fn is_stopping(&self) -> bool { ... }                // 防"停止中又启动"
    pub fn wait_stopping(&self, timeout: Duration) { ... }   // 启动前等待停止流程结束
}
```

启动线程的模式：

```rust
// src/fj200c_information/service.rs（示意）
let control: Arc<dyn IoControl> = ...;
let tx = fj200c_information_tx();          // broadcast::Sender 克隆
let handle = std::thread::spawn(move || {  // move：闭包拿走所有权（克隆引用）
    run_one_connection(connection_index, control, tx, stop);
});
RUNTIME.push(handle);
```

停止服务的统一骨架——`stop_in_background`（**异步停止**：HTTP 请求立即返回，后台线程 join 收尾）：

```rust
// src/common/service.rs —— 异步停止骨架
pub fn stop_in_background(
    runtime: &'static ServiceRuntime,
    running: &'static AtomicBool,
    set_stop: impl FnOnce() + Send + 'static,
    log_msg: &'static str,
) {
    runtime.set_stopping(true);     // 1. 置「停止进行中」标志
    set_stop();                     // 2. 触发工作线程退出（如 STOP_SIGNAL.store(true)）
    thread::spawn(move || {         // 3. 独立线程 join 所有句柄，不阻塞调用方
        for handle in runtime.drain() {
            let _ = handle.join();
        }
        running.store(false, Ordering::Relaxed);   // 4. 复位运行状态
        runtime.set_stopping(false);
        tracing::info!("{}", log_msg);
    });
}
```

要点：
- `thread::spawn` 的闭包用 `move` 关键字**捕获环境**（把变量所有权移入线程）——`tx` 是 `Sender` 的克隆，跨线程合法（`Sender<T>` 实现 `Send`）。
- 线程结束方式：原子布尔停止标志 + `join` 等待。
- **停止模式**：`set_stop` 置信号 → 线程循环看到标志 break → 后台 join 收尾。

### 2.11.2 共享状态的五件套

| 类型 | 用途 | 项目位置 |
|---|---|---|
| `Arc<T>` | 多线程共享只读数据 | `Arc<ChannelData>`（fj200c_main 事件字段） |
| `Arc<Mutex<T>>` | 多线程共享可变数据（互斥锁） | `ServiceRuntime.handles`、`ComControl.port` |
| `Arc<RwLock<T>>` / `RwLock<T>` | 读多写少共享数据 | `SharedData` 各字段（`RwLock<String>`） |
| `AtomicBool` / `AtomicU8` / `AtomicU32` | 单个数值的线程安全读写 | 停止标志、CSV 状态机、帧序号 |
| `ArcSwap<T>` | 无锁热替换（高频读） | `SharedPortData` 的 5×`ArcSwap<Fields>`、`QuadFrame` 槽位 |
| `OnceLock<T>` | 全局单例容器（只能 set 一次） | 所有 `xxx_tx()`、`config::global()`、`jwt` 密钥 |

```rust
// src/common/global_var.rs —— 全局 KV 存储（RwLock 的典型用法）
pub struct GlobalVar {
    inner: RwLock<HashMap<String, String>>,   // 读锁可并发，写锁独占
}

impl GlobalVar {
    pub fn set(&self, key: &str, value: &str) {
        let mut map = self.inner.write().unwrap();   // 写锁
        map.insert(key.to_string(), value.to_string());
    }
    pub fn get(&self, key: &str) -> Option<String> {
        let map = self.inner.read().unwrap();        // 读锁
        map.get(key).cloned()
    }
}
```

`OnceLock<T>`：**全局单例容器**——要么未初始化，要么已初始化，只能设置一次：

```rust
// src/fj200c_information/mod.rs —— 广播通道单例（由 event_broadcast! 宏生成）
pub static FJ200C_TX: OnceLock<broadcast::Sender<EventPayload>> = OnceLock::new();

pub fn fj200c_information_tx() -> broadcast::Sender<EventPayload> {
    FJ200C_TX
        .get_or_init(|| broadcast::channel(1024).0)  // 首次调用创建
        .clone()                                      // 克隆给调用者（计数+1）
}
```

### 2.11.3 broadcast：一对多广播通道（预序列化 Arc<str> 设计）

```rust
let (tx, _rx) = broadcast::channel(1024);   // 创建：发送端 + 接收端
let rx2 = tx.subscribe();                    // 每个新接收端从当前 Sender 派生
tx.send(event);                              // 广播给所有订阅者
rx.recv().await                              // 异步接收；Lagged → 丢旧消息
```

| 特性 | 说明 |
|---|---|
| 容量 | 1024 条；满了之后最旧的被丢弃，慢订阅者收 `Lagged` |
| 多接收者 | 每个 `subscribe()` 独立游标，互不干扰 |
| 自动清理 | 所有接收端 drop 后 send 返回 Err（会话线程据此退出） |
| 线程安全 | `Sender` 可克隆到任意线程（`std::thread` 里直接 `send`） |

**本项目最重要的性能设计——预序列化广播**（`src/common/ws.rs`）：

```rust
/// 广播通道中传输的事件载荷（预序列化的 JSON 文本）
pub type EventPayload = Arc<str>;

/// 序列化事件为广播载荷（生产端调用，只序列化一次）
pub fn serialize<E: Serialize>(event: &E) -> Result<EventPayload, serde_json::Error> {
    serde_json::to_string(event).map(Into::into)
}
```

数据流：采集线程 `ws::serialize(&event)` **一次**得到 `Arc<str>` → `tx.send(payload)`（广播只克隆指针）→ N 个 WS 连接各自 `payload.to_string()` 转发。**不再对每个客户端重复 `serde_json::to_string`**——高频帧场景下这是数量级的性能差异。事件对象在生产者处构建后立即序列化即被丢弃，零深克隆。

### 2.11.4 mpsc：多生产者单消费者命令通道

```rust
// src/fj200c_information/fj200c_information/session.rs（示意）：命令通道
// 服务启动时创建 (tx, rx)，rx 传给会话线程，tx 存入 COMMAND_TX 供 HTTP handler 发送
if let Some(cmd_rx) = COMMAND_RX.get() {
    if let Ok(cmd) = cmd_rx.lock().unwrap_or_else(|e| e.into_inner()).try_recv() {
        io.send(&cmd)?;   // 把命令写进串口/模拟器
    }
}
```

`try_recv` 是**非阻塞**取命令：会话线程每轮循环检查一次，有命令就发，没命令就继续收帧——命令与数据流互不阻塞。

### 2.11.5 ArcSwap：无锁热替换

```rust
// src/common/quad_frame.rs —— 四槽帧缓冲（const 泛型）
pub struct QuadFrame<const FRAME_LEN: usize> {
    slots: [ArcSwap<[u8; FRAME_LEN]>; SLOT_COUNT],  // 4 个槽位，主备双源
    seqs: [AtomicU32; SLOT_COUNT],                  // 序号（CAS 去重）
    active_source: AtomicU8,                        // 当前活跃源（主/备）
    primary_heartbeat: AtomicI64,                   // 主源心跳
}
// 读：read_slot() 无锁取当前帧（高频读场景，绝不停顿）
// 写：try_update() CAS + ArcSwap::store 原子替换（写者不阻塞读者）
```

```rust
// src/fj200c_main/fj200c_main/com.rs —— SharedPortData：5×最新帧 + 5×解码字段
pub struct SharedPortData {
    pub ecu_raw: LatestFrame<256>,
    pub adam4015_raw: LatestFrame<256>,
    // ... 5 路原始帧
    pub ecu_decoded: ArcSwap<EcuFields>,
    pub adam4015_decoded: ArcSwap<Adam4015Fields>,
    // ... 5 路解码字段
}
```

ArcSwap 用于"高频读、低频写"的热数据（最新帧），比 RwLock 更快（读完全无锁）。理解即可，不必深入。

### 2.11.6 登录限流：内存滑动窗口（src/common/rate_limit.rs）

```rust
const WINDOW: Duration = Duration::from_secs(60);      // 60 秒窗口
const MAX_ATTEMPTS: usize = 5;                         // 窗口内最多 5 次
const CLEANUP_THRESHOLD: usize = 10_000;               // 键数超阈值触发清理

struct Bucket { attempts: Vec<Instant> }               // 窗口内尝试时间戳
static LIMITER: OnceLock<Mutex<HashMap<String, Bucket>>> = OnceLock::new();

pub fn check_and_record(key: &str) -> Result<(), u64> {
    let now = Instant::now();
    let mut map = limiter().lock().unwrap_or_else(|e| e.into_inner());
    let bucket = map.entry(key.to_string()).or_insert_with(|| Bucket { attempts: Vec::new() });
    bucket.attempts.retain(|t| now.duration_since(*t) < WINDOW);  // 滑动窗口：清掉过期记录
    if bucket.attempts.len() >= MAX_ATTEMPTS {
        // 返回还需等待秒数
        let oldest = bucket.attempts.first().map(|t| now.duration_since(*t)).unwrap_or(WINDOW);
        return Err(WINDOW.as_secs().saturating_sub(oldest.as_secs()).max(1));
    }
    bucket.attempts.push(now);
    if map.len() >= CLEANUP_THRESHOLD { map.retain(...); }  // 防内存无限增长
    Ok(())
}
```

**架构地位**：广播通道是"采集线程 → WS 推送"的唯一桥梁，也是"HTTP 操作 → 采集线程"命令通道（mpsc，见上）的姊妹机制。限流是登录入口的安全防线——bcrypt 一次校验约 100ms CPU，无限制会被刷爆。

---

## 2.12 生命周期与借用：rust-analyzer 是你的眼睛

### 2.12.1 借用的两条规则

1. **一个可变借用 OR 多个只读借用**（不能同时可变+只读）。
2. **借用不能超过所有者的生命**（借用者存活期间，所有者不能被销毁）。

新手读项目代码时的实际体验：

```rust
let rows = Vec::new();
// 传给函数：传借用 &rows（不移动所有权），函数用完还能继续用 rows
let json = serde_json::to_string(&rows)?;
println!("{}", rows.len());   // ✓ rows 还在

// 传所有权（无 &）：rows 被"移动"，之后不能再碰
let json = serde_json::to_string(rows)?;
println!("{}", rows.len());   // ✗ 编译错误：rows 已被移动
```

### 2.12.2 生命周期标注 `'a`：编译器给你做证明题

```rust
// 含义："返回值活得不会比参数 a/b 更久"
fn foo<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() > b.len() { a } else { b } }
```

新手策略：
1. **90% 的情况编译器自动推断**（省略生命周期标注）。
2. 看到 `'static`：表示"整个程序生命周期"——`&'static str` 即字符串常量（`CONFIG_PATH` 那种）。
3. 看到 `&'a str`：只是编译器在检查借用时长，**不要慌，通常不用你改**。

### 2.12.3 新手最常遇到的借用错误与修法

| 编译错误 | 原因 | 修法 |
|---|---|---|
| `cannot borrow as mutable` | 已有不可变借用又取可变借用 | 缩小借用范围/用 RwLock |
| `value moved here` | 用了移动（无 &） | 加 `.clone()` 或改传 `&` |
| `borrow of moved value` | 移动后又用 | 结构体实现 Clone 后 clone |
| `lifetime may not live long enough` | 返回值借用被提前释放 | 返回 owned 值（String/Vec）而不是引用 |
| `captured variable in FnOnce` | 闭包用 move 后原变量失效 | 先 clone 一份再 move 进闭包 |

**实用技巧**：改完代码编译报借用错误，优先看编译器建议（它经常直接给出修法），其次参考**同一文件相邻代码的写法**——项目代码风格统一，照抄即可。

---

## 2.13 字符串与集合

### 2.13.1 String vs &str vs Arc<str>

```rust
let s: String = String::from("hello");    // 堆上、可增长、拥有所有权
let s2: String = "hello".to_string();     // 同上（常用写法）
let slice: &str = &s;                     // 借用视图，不可变
let lit: &'static str = "hello";          // 编译期字符串常量
let shared: Arc<str> = s.into();          // 引用计数共享（广播载荷就是它）
```

项目约定：**配置常量用 `&'static str`，动态数据用 `String`，函数参数尽量 `&str`**（可接受两者传入）。

本项目最妙的字符串设计是 **`Arc<str>` 预序列化广播**（见 2.11.3）：`String` 的堆数据用 `Arc` 共享后，**广播给 N 个客户端只克隆指针，数据只存在一份**。这是"字符串所有权"三个层次的完整应用：`&str`（借用视图）→ `String`（独占拥有）→ `Arc<str>`（共享拥有）。

```rust
// src/common/ws.rs —— 类型别名 + 一次序列化
pub type EventPayload = Arc<str>;
pub fn serialize<E: Serialize>(event: &E) -> Result<EventPayload, serde_json::Error> {
    serde_json::to_string(event).map(Into::into)   // String → Arc<str>
}
```

### 2.13.2 Vec：动态数组

```rust
let mut v = Vec::new();      // 创建空数组
v.push(1);                   // 追加
let x = v[0];                // 索引访问（越界 panic）
let first = v.first();       // Option 安全访问
v.iter()                     // 迭代器（借用）
v.len()                      // 长度
```

### 2.13.3 HashMap：键值对

```rust
let mut map: HashMap<String, String> = HashMap::new();
map.insert("k".into(), "v".into());
map.get("k")              // Option<&String>
map.entry("k").or_insert("default".into())   // 不存在才插入
```

项目使用场景：`global_var.rs` 的 KV 存储、`ftj1c` 的 `IpConfig`（32 个可选键：16 路 × 收发）、`rate_limit.rs` 的限流桶、`jwt.rs` 解码后的 Claims。

### 2.13.4 迭代器链（读代码必备）

```rust
// 项目模式：收集五路字段 CSV 行（src/fj200c_main/fj200c_main/types.rs 的思路）
let values: Vec<String> = fields.channels.iter()   // 遍历引用
    .map(|v| format!("{:.3}", v))                  // 转换（保留 3 位小数）
    .collect();                                    // 收集成 Vec
```

项目里迭代器链主要用于：列表转换、权限过滤（`permissions_for`）、配置解析、CSV 行拼接、小端字节转 ASCII（`common/utils.rs` 的 `little_endian_bytes_to_ascii`）。

---
