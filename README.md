# Art Tool

桌面端批量图像处理工具（Tauri + Rust + React）。

GitHub 仓库地址：<https://github.com/YUMI233666/Image_Tool>

当前版本聚焦可用的批处理主流程，支持本地文件批量输入、指定输出目录、任务进度追踪和结果报告导出。

## 当前功能

- 裁剪透明边缘（Trim Transparent）
  - 针对 PNG 图像，按 alpha 阈值裁剪到非透明像素边界。
  - 支持 padding（保留边距）。
  - 全透明图片会被跳过并记录原因。
- 图像格式转换（Format Convert）
  - 支持 png / jpg / webp / bmp / tiff。
  - 批处理时会按目标格式自动生成输出扩展名。
- 批处理能力
  - 支持批量文件或文件夹输入。
  - 支持递归子目录。
  - 支持最大并发配置。
  - 单文件失败不会中断整个任务。
  - 支持导出 JSON 报告。
- 扩展预留
  - 图像压缩（Compress）和图像修复（Repair）已完成接口与参数校验骨架，后续可直接接入真实算法。

## 技术栈

- 前端：React + Vite + Zustand
- 桌面容器：Tauri
- 后端：Rust
- 图像处理：image crate

## 环境要求

- Windows 10/11
- Node.js 24+
- npm 11+
- Rust stable（rustc/cargo）
- WebView2（Windows 通常已内置）

## 快速开始

如果你是从 GitHub 拉取项目，可以先执行：

```bash
git clone https://github.com/YUMI233666/Image_Tool.git
cd Image_Tool
```

1. 安装依赖

```bash
npm install
```

2. 启动开发模式

```bash
npm run tauri:dev
```

3. 运行后可在界面中进行试用

- 选择功能（建议先选“裁剪透明边缘”）
- 选择输入文件或输入文件夹
- 指定输出目录
- 配置参数（如 alphaThreshold、padding）
- 点击“开始处理”
- 在“结果汇总”查看成功/失败统计与失败样本

## 打包发布

执行：

```bash
npm run tauri:build
```

常见输出位置：

- 可执行文件：`src-tauri/target/release/`
- 安装包：`src-tauri/target/release/bundle/`

## 测试

Rust 测试：

```bash
npm run test:rust
```

或

```bash
cd src-tauri
cargo test
```

## 项目结构

```text
art tool/
├─ src/                     # 前端
│  ├─ components/           # 组件（功能选择、输入输出、队列、结果）
│  ├─ lib/                  # 类型定义与 Tauri API 封装
│  └─ store/                # 状态管理
├─ src-tauri/               # Rust + Tauri 后端
│  ├─ src/core/             # 处理器、批处理调度、文件发现、报告
│  ├─ src/commands/         # Tauri 命令入口
│  ├─ tests/                # Rust 测试
│  └─ icons/                # 打包资源
├─ docs/
│  ├─ processor-interface.md
│  └─ copilot-agent-plan.md
└─ README.md
```

## GitHub 上传建议

- 直接上传源码，不要上传以下目录（已在 `.gitignore` 配置）
  - `node_modules/`
  - `dist/`
  - `src-tauri/target/`
- 建议提交锁文件
  - `package-lock.json`
  - `src-tauri/Cargo.lock`

## 维护者首次上传流程

如果你本地是首次初始化并上传到 GitHub，可使用：

```bash
git init
git add .
git commit -m "chore: initialize Image Tool"
git branch -M main
git remote add origin https://github.com/YUMI233666/Image_Tool.git
git push -u origin main
```

如果远程已存在并需要更新：

```bash
git add .
git commit -m "feat: update Image Tool"
git push
```

## 已知说明

- 若 `cargo` 拉取依赖较慢，可配置镜像源后再执行构建。
- 首次 `tauri dev` 可能较慢（需要下载并编译 Rust 依赖）。
