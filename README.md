# Art Tool

Art Tool 是一个面向 Windows 的桌面端批量图像处理工具，支持批量导入、参数配置、任务队列、进度反馈和结果报告导出。

仓库地址：<https://github.com/YUMI233666/Image_Tool>

## 这个仓库适合谁

- 想批量处理图片的普通用户。
- 想在本地运行、调试或二次开发的开发者。

## 截图

（待补充截图）

## 功能总览

- 裁剪透明边缘（Trim Transparent）
  - 仅处理 PNG，按透明像素边界自动裁剪。
  - 支持透明阈值和保留边距。
- 手动裁剪（Manual Crop）
  - 支持比例锁定、逐张确认、应用到全部。
  - 实时显示裁剪后的像素尺寸。
- 图像格式转换（Format Convert）
  - 支持 PNG / JPG / WEBP 互转。
- 图像压缩（Compress）
  - 支持 JPG / PNG / WEBP。
  - 提供 `lossy`、`lossless`、`balanced` 模式和质量参数。
- 图像修复（Repair）
  - 支持 `auto`、`denoise`、`scratch`、`upscale`。
  - 可配置修复强度；`upscale` 支持放大倍数和锐化强度。
- 变换分辨率（Resolution Transform）
  - 可指定目标分辨率。
  - 目标更小自动缩放压缩，目标更大自动超分放大。
  - PNG 在目标比例不一致时会透明居中填充。
  - JPG / WEBP 在目标比例不一致时会保持原图比例适配。
  - 支持全局目标参数 + 单文件覆盖参数。
- 批量重命名（Rename）
  - 支持自定义命名与模板命名。
  - 模板变量：{name} {index} {date} {time} {ext}。
- 二次元超分（Anime Upscale）
  - 基于 realcugan-ncnn-vulkan sidecar 的二次元风格超分。
  - 支持倍率 2x/4x 与降噪等级 1-3（默认 3）。
- 工作流编排（Workflow）
  - 可添加多个处理步骤并按顺序执行。
  - 支持步骤级进度与失败定位。
- 批处理能力
  - 支持文件和文件夹输入。
  - 支持递归子目录。
  - 支持并发数设置。
  - 单文件失败不会中断整个任务。
  - 支持导出 JSON 报告。
- 文件列表缩略图预览
  - 支持缩略图与放大预览（Lightbox）。
  - 支持虚拟滚动，长列表更流畅。

## v1.3 更新

- 新增手动裁剪：比例锁定、逐张确认、应用到全部。
- 裁剪预览实时显示像素尺寸。
- 文件列表新增缩略图与放大预览（Lightbox），支持虚拟滚动。

## v1.1 更新

- 新增工作流编排（多步骤顺序执行）。
- 新增批量重命名（自定义/模板）。
- 增强步骤级进度与失败定位。

## 使用方式一：直接使用发布版本（推荐）

如果仓库的 Releases 页面已有安装包：

1. 打开 Releases 页面并下载最新安装包。
2. 安装后启动应用。
3. 按“使用流程”章节进行处理。

如果暂时没有发布包，请使用“方式二：从源码运行”。

## 使用方式二：从源码运行

### 1. 环境准备

- Windows 10 / 11
- Node.js 24+
- npm 11+
- Rust stable（包含 cargo）
- WebView2（Windows 一般已内置）
- 支持 Vulkan 的显卡驱动（用于二次元超分）

### 2. 克隆仓库

```bash
git clone https://github.com/YUMI233666/Image_Tool.git
cd Image_Tool
```

### 3. 安装依赖

```bash
npm install
```

### 3.1 下载二次元超分模型（可选但推荐）

Windows：

```bat
download-models.bat
```

Mac/Linux：

```bash
bash download-models.sh
```

### 4. 启动桌面应用（开发模式）

```bash
npm run tauri:dev
```

如果 PowerShell 出现执行策略导致的 `npm.ps1` 报错，可以改用：

```bash
npm.cmd run tauri:dev
```

## 使用流程（面向用户）

1. 选择运行模式：快捷模式或工作流模式。
2. 快捷模式：在“功能选择”中选处理功能；工作流模式：在“工作流编排”中添加步骤并选择处理器。
3. 在“参数设置”中配置当前步骤/功能参数。
4. （可选）在“批量重命名”面板启用命名规则。
5. （可选）若使用“二次元超分”，在参数区选择倍率与降噪等级（赛璐珞/伪厚涂建议 3，厚涂建议 1-2）。
6. 在“批处理输入”中选择输入文件或输入文件夹。
7. 选择输出目录。
8. 设置是否递归子目录、最大并发数。
9. 点击“开始处理”。
10. 在“任务队列”查看进度，在“结果汇总”查看成功/失败统计。
11. 需要时打开输出目录或报告文件。

## 输出与报告说明

- 输出文件会写入你指定的输出目录。
- 报告为 JSON 格式，包含：
  - 总任务统计（成功/失败/跳过/取消）
  - 每个文件的处理结果和消息
  - 开始/结束时间与耗时信息
- 报告路径：输出目录/.art-tool-tmp/reports/batch-report-<jobId>.json

## 二次元超分（模型放置）

1. 下载 realcugan-ncnn-vulkan 可执行文件与 models-se 模型。
2. 放置到以下目录结构：

```
src-tauri/
  binaries/
    realcugan-ncnn-vulkan.exe
    models-se/
      up2x-conservative.bin
      ...
```

说明：模型目录使用相对路径 `models-se`，与 sidecar 同目录。

### 下载 models-se（脚本推荐）

Windows：

```bat
download-models.bat
```

Mac/Linux：

```bash
bash download-models.sh
```

说明：脚本会从 Real-CUGAN 的最新 Release 里自动下载 models-se 压缩包并解压到 `src-tauri/binaries/models-se`。
如果下载到的是 `.7z`，请先安装 7-Zip（Windows）或 p7zip（Mac/Linux）。

### 下载 models-se（手动）

1. 打开 Real-CUGAN release 页面。
2. 下载 models-se 压缩包。
3. 解压后将 models-se 目录放入 `src-tauri/binaries/`。

## 常用命令

```bash
# 启动前端开发服务（仅前端）
npm run dev

# 启动桌面应用（Tauri）
npm run tauri:dev

# 构建前端产物
npm run build

# 打包桌面应用
npm run tauri:build

# 运行 Rust 测试
npm run test:rust
```

打包后常见输出目录：

- 可执行文件：`src-tauri/target/release/`
- 安装包：`src-tauri/target/release/bundle/`

## 常见问题（FAQ）

### 1) 为什么某些文件没有处理成功？

可能原因：

- 文件已损坏或格式不受支持。
- 输出目录权限不足。
- 当前功能对该格式有限制（例如透明裁剪仅支持 PNG）。

可在“结果汇总”和导出报告中查看具体错误消息。

### 2) 目录处理时文件太多怎么办？

- 先关闭“递归处理子目录”缩小处理范围。
- 适当降低最大并发数，减少资源占用。

### 3) 首次启动很慢正常吗？

正常。首次运行会编译和下载依赖，后续会快很多。

## 反馈与贡献

欢迎通过 GitHub Issue 反馈问题或提出建议。提交问题时建议附带：

- 系统版本（Windows 版本）
- 操作步骤
- 报错截图或日志
- 报告文件中的失败样本信息

如果你愿意提交 PR，也欢迎直接贡献代码。

## 许可证

本项目使用 MIT License。
