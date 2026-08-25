# VitePress 从零到一构建文档网站完全指南

> 万字长文，手把手带你从零开始搭建一个属于自己的文档网站


## 目录

1. [为什么选择 VitePress](#一为什么选择-vitepress)
2. [环境准备](#二环境准备)
3. [项目初始化](#三项目初始化)
4. [项目结构与核心文件](#四项目结构与核心文件)
5. [配置文件详解](#五配置文件详解)
6. [编写文档内容](#六编写文档内容)
7. [导航栏与侧边栏配置](#七导航栏与侧边栏配置)
8. [自定义主题与样式](#八自定义主题与样式)
9. [本地构建与预览](#九本地构建与预览)
10. [部署到 GitHub Pages](#十部署到-github-pages)
11. [进阶功能](#十一进阶功能)
12. [最佳实践与性能优化](#十二最佳实践与性能优化)
13. [常见问题与排错](#十三常见问题与排错)


## 一、为什么选择 VitePress

VitePress 是由 Vue 团队开发的静态站点生成器（SSG）。它基于 Vite 和 Vue 3 构建，专为编写技术文档而设计。

### 1.1 核心优势

**🚀 极致的开发体验**

VitePress 基于 Vite，冷启动速度极快。实测对比传统 SSG 工具，一个包含 50 个页面的文档项目，传统工具首次启动需要 4-6 秒，而 VitePress 仅需约 700 毫秒；热更新更是达到惊人的 50-100 毫秒。

**📝 Markdown 与 Vue 的完美融合**

你可以在 Markdown 文件中直接使用 Vue 组件，让文档拥有交互性。这对于展示代码示例、动态演示等功能非常有用。

**⚡ 开箱即用的性能优化**

VitePress 采用静态 HTML 预渲染加 SPA 导航的混合架构，首屏加载飞快。初次访问时页面是静态的、预渲染的 HTML，能实现极快的加载速度和最佳的 SEO。

**🎯 专注于文档**

VitePress 的默认主题专为文档场景优化，开箱即用，包含导航栏、侧边栏、搜索等功能。

### 1.2 VitePress vs 其他方案

| 特性 | VitePress | VuePress | Docsify | Docusaurus |
|------|-----------|----------|---------|------------|
| 构建方式 | 静态生成 | 静态生成 | 运行时渲染 | 静态生成 |
| 速度 | 极快 | 中等 | 快 | 中等 |
| Vue 支持 | 原生 | 原生 | 有限 | 有限 |
| 学习曲线 | 低 | 中 | 极低 | 中 |
| 适合场景 | 技术文档 | 博客/文档 | 简单文档 | 大型项目 |


## 二、环境准备

### 2.1 安装 Node.js

VitePress 要求 Node.js **20 及以上版本**。

**验证 Node.js 版本：**

```bash
node -v
# 应该输出 v20.x.x 或更高版本
```

如果版本低于 20，请前往 [Node.js 官网](https://nodejs.org/) 下载最新的 LTS 版本。

### 2.2 选择包管理器

你可以使用 npm、pnpm、yarn 或 bun。本文档推荐使用 **pnpm**，它的安装速度更快，磁盘占用更少。

安装 pnpm（如果尚未安装）：

```bash
npm install -g pnpm
```

### 2.3 选择代码编辑器

推荐使用 **VS Code**，并安装官方 Vue 扩展：
- [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
- 支持 Markdown 语法高亮的插件（如 Markdown All in One）

### 2.4 创建项目目录

首先，创建一个新的项目文件夹（名称自定）：

```bash
mkdir my-docs
cd my-docs
```


## 三、项目初始化

### 3.1 使用初始化向导（推荐）

VitePress 提供了命令行设置向导，可以帮你快速搭建一个基础项目。

运行以下命令：

```bash
pnpm create vitepress@latest
```

或者使用 npx：

```bash
npx vitepress init
```

### 3.2 回答向导问题

向导会问你几个简单的问题：

```
┌  Welcome to VitePress!
│
◇  Where should VitePress initialize the config?
│  ./docs
│
◇  Where should VitePress look for your markdown files?
│  ./docs
│
◇  Site title:
│  My Awesome Docs
│
◇  Site description:
│  A VitePress documentation site
│
◇  Theme:
│  Default Theme
│
◇  Use TypeScript for config and theme files?
│  Yes
│
◇  Add VitePress npm scripts to package.json?
│  Yes
│
◇  Add a prefix for VitePress npm scripts?
│  Yes
│
◇  Prefix for VitePress npm scripts:
│  docs
│
└  Done! Now run pnpm run docs:dev and start writing.
```

### 3.3 手动安装（可选）

如果你更倾向于手动安装，可以按以下步骤操作：

**第一步：初始化 package.json**

```bash
pnpm init
```

**第二步：安装 VitePress**

```bash
pnpm add -D vitepress
```

> **注意**：VitePress 是仅 ESM 的软件包，确保 `package.json` 包含 `"type": "module"`。

**第三步：创建目录结构**

```bash
mkdir -p docs/.vitepress
```

**第四步：创建配置文件**

在 `docs/.vitepress/config.mts` 中创建配置文件。

**第五步：添加 npm 脚本**

在 `package.json` 中添加：

```json
{
  "scripts": {
    "docs:dev": "vitepress dev docs",
    "docs:build": "vitepress build docs",
    "docs:preview": "vitepress preview docs"
  }
}
```


## 四、项目结构与核心文件

初始化完成后，项目结构如下：

```
my-docs/
├── docs/                      # 文档根目录
│   ├── .vitepress/            # VitePress 配置目录
│   │   └── config.mts         # 站点配置文件
│   ├── api-examples.md        # API 示例页面
│   ├── markdown-examples.md   # Markdown 示例页面
│   └── index.md               # 网站首页
├── package.json               # 项目依赖配置
└── pnpm-lock.yaml             # 依赖锁定文件
```

### 4.1 各目录/文件说明

| 路径 | 说明 |
|------|------|
| `docs/` | VitePress 站点的项目根目录 |
| `docs/.vitepress/` | 配置文件、开发服务器缓存、构建输出和主题自定义代码的位置 |
| `docs/.vitepress/cache/` | 开发服务器缓存（应加入 `.gitignore`） |
| `docs/.vitepress/dist/` | 生产构建输出（应加入 `.gitignore`） |
| `docs/index.md` | 网站首页 |
| `docs/**/*.md` | 其他文档页面 |

### 4.2 .gitignore 配置

如果使用 Git 进行版本控制，建议在项目根目录创建 `.gitignore` 文件：

```gitignore
node_modules/
docs/.vitepress/cache/
docs/.vitepress/dist/
.DS_Store
```


## 五、配置文件详解

配置文件（`docs/.vitepress/config.mts`）是 VitePress 站点的核心，它控制着站点的几乎所有方面。

### 5.1 基础配置

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress'

export default defineConfig({
  // 站点级选项
  title: 'My Awesome Docs',
  description: 'A VitePress documentation site',
  
  // 网站语言
  lang: 'zh-CN',
  
  // 网站图标
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }]
  ],
  
  // 主题配置
  themeConfig: {
    // 主题级选项
  }
})
```

### 5.2 站点级配置选项

| 选项 | 类型 | 说明 |
|------|------|------|
| `title` | `string` | 网站标题 |
| `description` | `string` | 网站描述（用于 SEO） |
| `lang` | `string` | 网站语言 |
| `base` | `string` | 部署的基础路径，默认为 `/` |
| `head` | `Array` | 额外添加到 `<head>` 中的标签 |
| `lastUpdated` | `boolean` | 是否显示最后更新时间 |
| `ignoreDeadLinks` | `boolean` | 是否忽略死链接检查 |

### 5.3 完整配置示例

```typescript
// docs/.vitepress/config.mts
import { defineConfig } from 'vitepress'

export default defineConfig({
  // ===== 站点级配置 =====
  title: '我的技术文档',
  description: '从零到一的 VitePress 学习笔记',
  lang: 'zh-CN',
  
  // 如果部署到 https://username.github.io/repo/，需要设置 base
  // base: '/repo/',
  
  head: [
    ['link', { rel: 'icon', href: '/logo.svg' }],
    ['meta', { name: 'theme-color', content: '#42b883' }],
    ['meta', { property: 'og:title', content: '我的技术文档' }]
  ],
  
  // 最后更新时间
  lastUpdated: true,
  
  // ===== 主题配置 =====
  themeConfig: {
    // 网站标志
    logo: '/logo.svg',
    
    // 导航栏
    nav: [
      { text: '首页', link: '/' },
      { text: '指南', link: '/guide/' },
      { text: 'API', link: '/api/' },
      { 
        text: '相关链接',
        items: [
          { text: 'GitHub', link: 'https://github.com' },
          { text: 'VitePress', link: 'https://vitepress.dev' }
        ]
      }
    ],
    
    // 侧边栏
    sidebar: {
      '/guide/': [
        {
          text: '开始',
          items: [
            { text: '介绍', link: '/guide/' },
            { text: '快速开始', link: '/guide/getting-started' }
          ]
        },
        {
          text: '进阶',
          items: [
            { text: '配置', link: '/guide/configuration' },
            { text: '部署', link: '/guide/deployment' }
          ]
        }
      ]
    },
    
    // 社交链接
    socialLinks: [
      { icon: 'github', link: 'https://github.com/your-username' }
    ],
    
    // 页脚
    footer: {
      message: '基于 VitePress 构建',
      copyright: 'Copyright © 2026'
    },
    
    // 搜索（本地搜索）
    search: {
      provider: 'local'
    },
    
    // 编辑链接
    editLink: {
      pattern: 'https://github.com/your-username/repo/edit/main/docs/:path',
      text: '在 GitHub 上编辑此页'
    }
  }
})
```


## 六、编写文档内容

### 6.1 Markdown 基础

VitePress 使用标准的 Markdown 语法，并在此基础上进行了扩展。

**基本语法：**

```markdown
# 一级标题
## 二级标题
### 三级标题

**粗体文本** *斜体文本*

- 无序列表项 1
- 无序列表项 2

1. 有序列表项 1
2. 有序列表项 2

[链接文本](https://example.com)

![图片描述](/image.png)

`行内代码`

​```javascript
// 代码块
console.log('Hello World')
​```
```

### 6.2 Frontmatter（前置元数据）

你可以在 Markdown 文件顶部使用 YAML Frontmatter 来设置页面的元数据：

```markdown
---
title: 页面标题
description: 页面描述
sidebar: auto
editLink: true
lastUpdated: true
---

# 页面内容开始
```

常用 Frontmatter 选项：

| 选项 | 类型 | 说明 |
|------|------|------|
| `title` | `string` | 页面标题（覆盖全局配置） |
| `description` | `string` | 页面描述 |
| `layout` | `'doc' | 'page' | 'home'` | 页面布局 |
| `sidebar` | `boolean | 'auto'` | 是否显示侧边栏 |
| `editLink` | `boolean` | 是否显示编辑链接 |
| `lastUpdated` | `boolean` | 是否显示最后更新时间 |

### 6.3 自定义容器（Custom Containers）

VitePress 支持自定义容器，用于突出显示重要信息：

```markdown
::: info
这是一条信息提示。
:::

::: tip
这是一条建议提示。
:::

::: warning
这是一条警告提示。
:::

::: danger
这是一条危险提示。
:::

::: details 点击展开详情
这里是详细信息内容。
:::
```

渲染效果分别是信息、建议、警告、危险和可折叠详情容器。

### 6.4 在 Markdown 中使用 Vue 组件

这是 VitePress 最强大的功能之一——你可以在 Markdown 中直接使用 Vue 组件：

```markdown
# 交互式示例

<script setup>
import { ref } from 'vue'
const count = ref(0)
</script>

<button @click="count++">点击计数：{{ count }}</button>
```

### 6.5 首页设计（Hero 布局）

VitePress 支持特殊的首页布局，通过 Frontmatter 设置 `layout: home`：

```markdown
---
layout: home

hero:
  name: "我的技术文档"
  text: "从零到一构建知识库"
  tagline: 使用 VitePress 打造高效、美观的文档站点
  image:
    src: /logo.svg
    alt: Logo
  actions:
    - theme: brand
      text: 快速开始 →
      link: /guide/getting-started
    - theme: alt
      text: 在 GitHub 查看
      link: https://github.com

features:
  - icon: 🚀
    title: 极速开发
    details: 基于 Vite，冷启动和热更新快到飞起
  - icon: 📝
    title: Markdown 优先
    details: 用 Markdown 写作，天然支持 Vue 组件
  - icon: 🎨
    title: 开箱即用
    details: 默认主题专为文档优化，无需额外配置
---
```


## 七、导航栏与侧边栏配置

### 7.1 导航栏（Nav）

导航栏配置在 `themeConfig.nav` 中：

```typescript
themeConfig: {
  nav: [
    { text: '首页', link: '/' },
    { text: '指南', link: '/guide/' },
    { text: 'API 参考', link: '/api/' },
    {
      text: '更多',
      items: [
        { text: '关于', link: '/about' },
        { text: '博客', link: '/blog' }
      ]
    }
  ]
}
```

**多级菜单**：导航栏最多建议展示到三级菜单，更深层级的内容应通过侧边栏组织。

### 7.2 侧边栏（Sidebar）

侧边栏是文档的主要导航模块。

**简单侧边栏（数组形式）** ：

```typescript
themeConfig: {
  sidebar: [
    {
      text: '入门',
      items: [
        { text: '介绍', link: '/guide/' },
        { text: '安装', link: '/guide/installation' },
        { text: '快速开始', link: '/guide/getting-started' }
      ]
    },
    {
      text: '进阶',
      items: [
        { text: '配置', link: '/guide/configuration' },
        { text: '部署', link: '/guide/deployment' }
      ]
    }
  ]
}
```

**多侧边栏（对象形式）** ：为不同路径配置不同的侧边栏：

```typescript
themeConfig: {
  sidebar: {
    '/guide/': [
      {
        text: '指南',
        items: [
          { text: '介绍', link: '/guide/' },
          { text: '快速开始', link: '/guide/getting-started' }
        ]
      }
    ],
    '/api/': [
      {
        text: 'API',
        items: [
          { text: '核心 API', link: '/api/core' },
          { text: '工具函数', link: '/api/utils' }
        ]
      }
    ]
  }
}
```

**可折叠分组**：

```typescript
{
  text: '高级主题',
  collapsed: true,  // 默认折叠
  items: [
    { text: '自定义主题', link: '/advanced/theme' },
    { text: '插件开发', link: '/advanced/plugins' }
  ]
}
```

### 7.3 自动生成侧边栏

对于大型文档，可以借助社区插件 `vitepress-sidebar` 自动生成侧边栏。


## 八、自定义主题与样式

### 8.1 自定义 CSS

你可以通过覆盖 CSS 变量来定制默认主题的样式。

创建 `docs/.vitepress/theme/style.css`：

```css
/* docs/.vitepress/theme/style.css */
:root {
  /* 品牌色 */
  --vp-c-brand-1: #42b883;
  --vp-c-brand-2: #3aa776;
  
  /* 字体 */
  --vp-font-family-base: 'Inter', 'Microsoft YaHei', sans-serif;
  --vp-font-family-mono: 'JetBrains Mono', monospace;
  
  /* 代码块 */
  --vp-code-block-bg: #1e1e2e;
}

/* 自定义首页样式 */
.VPHome {
  background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
}
```

然后在主题入口文件中引入：

```typescript
// docs/.vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme'
import './style.css'

export default DefaultTheme
```

### 8.2 使用自定义主题

创建主题入口文件 `docs/.vitepress/theme/index.ts` 即可启用自定义主题：

```typescript
// docs/.vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import MyComponent from './components/MyComponent.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // 注册全局组件
    app.component('MyComponent', MyComponent)
  }
} satisfies Theme
```

### 8.3 布局插槽

默认主题的 `<Layout/>` 组件提供了多个插槽，可以在页面特定位置注入内容：

```typescript
// docs/.vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme'

export default {
  extends: DefaultTheme,
  Layout: {
    'aside-top': () => {
      // 在侧边栏顶部添加内容
    },
    'doc-before': () => {
      // 在文档内容前添加内容
    }
  }
}
```

### 8.4 自定义布局

如果需要完全自定义页面布局，可以创建自定义布局组件：

```vue
<!-- docs/.vitepress/theme/MyLayout.vue -->
<template>
  <div class="my-layout">
    <header>我的自定义头部</header>
    <main>
      <Content />
    </main>
    <footer>我的自定义页脚</footer>
  </div>
</template>
```

然后在主题入口文件中使用：

```typescript
// docs/.vitepress/theme/index.ts
import MyLayout from './MyLayout.vue'

export default {
  Layout: MyLayout
}
```


## 九、本地构建与预览

### 9.1 启动开发服务器

在项目根目录运行：

```bash
pnpm run docs:dev
```

开发服务器默认运行在 `http://localhost:5173`。当你修改 Markdown 文件时，页面会实时热更新。

### 9.2 构建生产版本

```bash
pnpm run docs:build
```

构建产物默认输出到 `docs/.vitepress/dist` 目录。

### 9.3 本地预览生产版本

```bash
pnpm run docs:preview
```

预览服务器默认运行在 `http://localhost:4173`。这是检查生产版本在本地环境中是否正常的有效方法。

### 9.4 配置预览端口

```json
{
  "scripts": {
    "docs:preview": "vitepress preview docs --port 8080"
  }
}
```


## 十、部署到 GitHub Pages

### 10.1 创建 GitHub 仓库

1. 登录 GitHub，点击右上角的 **+** → **New repository**
2. 仓库名称自定（如 `my-docs`）
3. 选择 **Public**（免费使用 GitHub Pages 必须公开）
4. 不要初始化 README（如果你已有本地项目）
5. 点击 **Create repository**

### 10.2 推送代码到 GitHub

```bash
git init
git add .
git commit -m "Initial commit"
git remote add origin https://github.com/你的用户名/仓库名.git
git branch -M main
git push -u origin main
```

### 10.3 配置 GitHub Actions 工作流

在项目根目录创建 `.github/workflows/deploy.yml` 文件：

```yaml
# .github/workflows/deploy.yml
# 构建 VitePress 站点并将其部署到 GitHub Pages 的示例工作流程
name: Deploy VitePress site to Pages

on:
  # 在针对 main 分支的推送上运行
  push:
    branches: [main]
  # 允许从 Actions 选项卡手动运行此工作流程
  workflow_dispatch:

# 设置 GITHUB_TOKEN 的权限，以允许部署到 GitHub Pages
permissions:
  contents: read
  pages: write
  id-token: write

# 只允许同时进行一次部署
concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  # 构建工作
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
      
      - name: Setup Pages
        uses: actions/configure-pages@v4
      
      - name: Install dependencies
        run: npm ci
      
      - name: Build with VitePress
        run: npm run docs:build
      
      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: docs/.vitepress/dist

  # 部署工作
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    needs: build
    runs-on: ubuntu-latest
    name: Deploy
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

> **注意**：如果使用 pnpm，需要取消注释 `pnpm/action-setup` 部分。

### 10.4 启用 GitHub Pages

1. 进入仓库的 **Settings** → **Pages**
2. 将 **Source** 设置为 **GitHub Actions**
3. 保存设置

### 10.5 触发部署

推送代码到 `main` 分支后，GitHub Actions 会自动运行部署工作流。

```bash
git add .
git commit -m "更新文档"
git push
```

### 10.6 查看部署结果

1. 进入仓库的 **Actions** 选项卡，查看工作流运行状态
2. 运行成功后，进入 **Settings** → **Pages**，查看分配的访问地址
3. 访问 `https://你的用户名.github.io/仓库名`

### 10.7 关于 base 路径的重要说明

如果网站部署在子路径（如 `https://username.github.io/repo/`），必须在 VitePress 配置中设置 `base` 选项：

```typescript
export default defineConfig({
  base: '/repo/',  // 替换为你的仓库名
  // ... 其他配置
})
```

如果打开网站发现样式丢失，通常就是 `base` 路径配置不正确。


## 十一、进阶功能

### 11.1 本地搜索

VitePress 默认主题支持本地全文搜索：

```typescript
themeConfig: {
  search: {
    provider: 'local',
    options: {
      translations: {
        button: {
          buttonText: '搜索文档',
          buttonAriaLabel: '搜索'
        },
        modal: {
          noResultsText: '无法找到相关结果',
          resetButtonTitle: '清除查询条件',
          footer: {
            selectText: '选择',
            navigateText: '切换'
          }
        }
      }
    }
  }
}
```

### 11.2 Algolia 搜索

对于大型文档网站，可以使用 Algolia 提供更强大的搜索体验：

```typescript
themeConfig: {
  search: {
    provider: 'algolia',
    options: {
      appId: '你的 App ID',
      apiKey: '你的 API Key',
      indexName: '你的索引名称'
    }
  }
}
```

### 11.3 国际化（i18n）

VitePress 内置了国际化支持：

**目录结构：**

```
docs/
├── index.md          # 默认语言（英文）
├── zh/
│   ├── index.md      # 中文首页
│   └── guide.md      # 中文指南
└── ja/
    ├── index.md      # 日文首页
    └── guide.md      # 日文指南
```

**配置文件：**

```typescript
export default defineConfig({
  locales: {
    '/': {
      lang: 'en-US',
      title: 'My Docs',
      description: 'Documentation site'
    },
    '/zh/': {
      lang: 'zh-CN',
      title: '我的文档',
      description: '文档站点'
    }
  },
  themeConfig: {
    locales: {
      '/': {
        nav: [{ text: 'Home', link: '/' }]
      },
      '/zh/': {
        nav: [{ text: '首页', link: '/zh/' }]
      }
    }
  }
})
```

### 11.4 Markdown 扩展

**语法高亮**：VitePress 使用 Shiki 在 Markdown 代码块中实现语法高亮，支持多种编程语言。

**表格**：

```markdown
| 列1 | 列2 | 列3 |
|-----|-----|-----|
| 内容1 | 内容2 | 内容3 |
```

**Emoji**：直接在 Markdown 中使用 `:emoji:` 语法。

**数学公式**：可以通过插件支持 LaTeX 数学公式。

### 11.5 使用插件

VitePress 生态中有许多实用插件：

- `vitepress-plugin-obsidian`：支持 Obsidian 风格的 Markdown 语法
- `vitepress-plugin-responsive-images`：自动提供响应式图片
- `vitepress-sidebar`：自动生成侧边栏

安装和使用示例：

```bash
pnpm add -D vitepress-plugin-obsidian
```

```typescript
// docs/.vitepress/config.mts
import { obsidianMarkdownPlugin } from 'vitepress-plugin-obsidian'

export default defineConfig({
  markdown: {
    config: (md) => {
      md.use(obsidianMarkdownPlugin)
    }
  }
})
```


## 十二、最佳实践与性能优化

### 12.1 内容组织

**📁 合理的目录结构**：

```
docs/
├── index.md              # 首页
├── guide/                # 指南章节
│   ├── index.md          # 指南首页
│   ├── getting-started.md
│   └── configuration.md
├── api/                  # API 章节
│   ├── index.md
│   └── core.md
└── .vitepress/
    └── config.mts
```

### 12.2 图片优化

- 使用现代图片格式（WebP、AVIF）
- 压缩图片大小
- 使用 CDN 加速图片加载

### 12.3 缓存策略

VitePress 生产版本对静态资源使用哈希文件名（如 `app.4f283b18.js`），可以安全地使用强缓存：

```
Cache-Control: max-age=31536000, immutable
```

### 12.4 SEO 优化

- 为每个页面设置合适的 `title` 和 `description`
- 使用语义化的 URL 结构
- 生成 sitemap.xml
- 配置 robots.txt

### 12.5 维护与更新

- 定期审核和更新文档内容
- 利用 Git 进行版本管理
- 使用 GitHub Actions 实现自动化部署

### 12.6 开发工作流建议

一个高效的工作流程包括：

1. **内容创作**：使用熟悉的工具撰写文档
2. **格式转换**：导出为 Markdown 格式
3. **本地开发**：使用 VitePress 预览效果
4. **版本控制**：提交到 Git 仓库
5. **自动部署**：通过 CI/CD 自动发布


## 十三、常见问题与排错

### Q1：开发服务器启动失败

**检查 Node.js 版本**：确保 Node.js 版本 >= 20。

**检查包管理器**：确保使用正确的包管理器安装依赖。

### Q2：样式丢失

**检查 base 路径**：如果部署在子路径，确保 `base` 配置正确。

**检查资源路径**：图片等静态资源使用绝对路径（以 `/` 开头）。

### Q3：GitHub Actions 部署失败

**检查工作流文件**：确保 `.github/workflows/deploy.yml` 语法正确。

**检查分支名称**：确保 `on.push.branches` 中的分支名与你的默认分支一致。

**检查权限**：确保仓库 Settings → Pages 中 Source 设置为 "GitHub Actions"。

### Q4：页面 404

**检查文件路径**：确保 Markdown 文件的链接路径正确。

**检查 base 配置**：子路径部署时需要正确配置 `base`。

### Q5：热更新不生效

**检查文件监听**：确保开发服务器正在运行。

**重启服务器**：有时配置文件修改后需要重启开发服务器。


## 总结

通过本教程，你已经学会了：

1. **环境准备**：安装 Node.js 和包管理器
2. **项目初始化**：使用 VitePress 向导快速创建项目
3. **配置站点**：掌握 `config.mts` 的核心配置项
4. **编写内容**：使用 Markdown 和 Vue 组件撰写文档
5. **自定义样式**：通过 CSS 和主题定制外观
6. **本地开发**：使用 dev 服务器实时预览
7. **部署上线**：通过 GitHub Actions 自动部署到 GitHub Pages

VitePress 的强大之处在于它将 **Markdown 的简洁** 与 **Vue 的灵活性** 完美结合，让你能够专注于内容创作，而不必被技术细节所困扰。

现在，开始构建属于你自己的文档网站吧！🚀


**延伸阅读**：
- [VitePress 官方文档](https://vitepress.dev)
- [VitePress GitHub 仓库](https://github.com/vuejs/vitepress)
- [Vue 3 官方文档](https://vuejs.org)