# Rust Web 全栈项目

这是一个使用 Rust + Axum 后端和 Vue3 + Vite 前端的全栈管理系统。

## 项目结构

```
RustWeb/
├── src/                    # Rust 后端源码
│   ├── main.rs            # 主入口文件
│   ├── config.rs          # 配置管理
│   ├── database.rs        # 数据库连接
│   ├── models.rs          # 数据模型
│   ├── services.rs        # 业务逻辑
│   ├── handlers.rs        # HTTP 处理器
│   ├── routes.rs          # 路由定义
│   ├── middleware.rs      # 中间件
│   └── utils/             # 工具模块
│       └── jwt.rs         # JWT 处理
├── migrations/            # 数据库迁移
├── frontend/              # Vue3 前端
│   ├── src/
│   │   ├── api/           # API 服务
│   │   ├── components/    # Vue 组件
│   │   ├── router/        # 路由配置
│   │   ├── stores/        # 状态管理
│   │   ├── types/         # TypeScript 类型
│   │   ├── views/         # 页面组件
│   │   ├── App.vue        # 主应用组件
│   │   └── main.ts        # 应用入口
│   ├── package.json       # 前端依赖
│   └── vite.config.ts     # Vite 配置
├── Cargo.toml            # Rust 依赖配置
├── start.sh              # Linux/Mac 启动脚本
├── start.bat             # Windows 启动脚本
└── README.md             # 项目说明
```

## 后端特性

- **Axum Web 框架**: 高性能的 Rust Web 框架
- **PostgreSQL 数据库**: 使用 SQLx 进行数据库操作
- **JWT 认证**: 安全的用户认证系统
- **模块化架构**: 清晰的代码组织结构
- **错误处理**: 统一的错误处理机制
- **数据验证**: 使用 validator 进行数据验证
- **日志系统**: 使用 tracing 进行日志记录

## API 端点

### 用户管理
- `POST /api/users/register` - 用户注册
- `POST /api/users/login` - 用户登录
- `GET /api/users/profile` - 获取用户信息

### 文章管理
- `GET /api/posts` - 获取文章列表
- `POST /api/posts` - 创建文章
- `GET /api/posts/:id` - 获取文章详情
- `PUT /api/posts/:id` - 更新文章
- `DELETE /api/posts/:id` - 删除文章

## 环境变量

创建 `.env` 文件：

```env
PORT=3000
DATABASE_URL=postgres://postgres:password@localhost/rustweb
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
JWT_EXPIRATION=86400
RUST_LOG=info
```

## 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+
- PostgreSQL 12+
- Yarn 包管理器

### 安装步骤

1. **克隆项目**
```bash
git clone <repository-url>
cd RustWeb
```

2. **配置环境变量**
```bash
# 复制环境变量示例文件
cp .env.example .env

# 编辑 .env 文件，配置数据库连接等信息
```

3. **设置数据库**
```bash
# 创建数据库
createdb rustweb

# 或者使用 psql
psql -U postgres -c "CREATE DATABASE rustweb;"
```

4. **安装依赖**
```bash
# 安装 Rust 依赖
cargo build

# 安装前端依赖
cd frontend
yarn install
```

5. **运行项目**

**方法一：使用启动脚本**
```bash
# Linux/Mac
chmod +x start.sh
./start.sh

# Windows
start.bat
```

**方法二：分别启动**
```bash
# 终端 1：启动后端
cargo run

# 终端 2：启动前端
cd frontend
yarn dev
```

6. **访问应用**
- 前端地址：http://localhost:5173
- 后端 API：http://localhost:3000
- API 文档：http://localhost:3000/health

## 前端开发

前端使用 Vue3 + Vite + TypeScript 构建，包含：

- 现代化的 UI 设计
- 响应式布局
- 用户认证
- 文章管理界面
- 实时数据更新

## 开发指南

### 添加新的 API 端点

1. 在 `models.rs` 中定义数据模型
2. 在 `services.rs` 中实现业务逻辑
3. 在 `handlers.rs` 中创建处理器
4. 在 `routes.rs` 中注册路由

### 数据库迁移

使用 SQLx CLI 进行数据库迁移：

```bash
cargo install sqlx-cli
sqlx migrate add <migration_name>
sqlx migrate run
```

## 部署

### 后端部署

1. 构建生产版本：`cargo build --release`
2. 设置生产环境变量
3. 运行数据库迁移
4. 启动服务

### 前端部署

1. 构建生产版本：`npm run build`
2. 部署到静态文件服务器

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
