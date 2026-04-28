# Processor 接口规范

本文档用于约束 Art Tool 的处理器扩展接入方式，适用于图像格式转换、图像压缩、图像修复、分辨率变换与批量重命名等功能。

## 目标
- 保证新功能可被统一调度。
- 保证参数校验和错误信息格式统一。
- 保证批处理过程中的单文件失败不会中断全局任务。

## 接口定义
处理器需要实现 Processor trait，包含三个核心方法：
1. descriptor()
2. validate(params)
3. process(context)

约束如下：
- descriptor.id 必须全局唯一，建议使用 kebab-case。
- descriptor.enabled 用于控制前端是否可执行该功能。
- validate 只负责参数合法性，不做耗时 IO。
- process 必须是单文件处理逻辑，不能依赖全局可变状态。

## 参数模型建议
- 图像格式转换：targetFormat、background、quality（可选）。
- 图像压缩：quality、mode（lossy/lossless/balanced）。
- 图像修复：mode（auto/denoise/scratch/upscale）、strength、upscaleFactor（2-4，仅 upscale 模式生效）、upscaleSharpness（1-100，仅 upscale 模式生效）。
- 变换分辨率：targetWidth、targetHeight、upscaleSharpness（1-100）、fileOverrides（按输入路径覆盖目标分辨率）。
- 批量重命名：由批处理请求的 renameConfig 提供规则（见下方工作流与命名规则）。

## 工作流与重命名规则

批处理请求支持两种模式：

1. 快捷模式（单处理器）：
	- 使用 processorId + params 直接运行。
2. 工作流模式（多步骤）：
	- 使用 workflowSteps 依序执行多个处理器。

### 工作流步骤结构

- workflowSteps: Array<WorkflowStep>
  - stepId: string
  - processorId: string
  - params: object

处理器依旧是“单文件处理逻辑”，工作流只是对单文件执行多个处理器的编排。

### 重命名规则

批处理请求支持 renameConfig：

- enabled: bool
- mode: custom | template
- customName?: string
- template?: string
- startIndex?: number (默认 1)
- indexPadding?: number (默认 0)

模板变量：{name} {index} {date} {time} {ext}

## 错误处理规范
- 参数错误：返回 Validation。
- 暂未实现：返回 Unsupported。
- 文件读写错误：返回 Io。
- 图像解码错误：返回 Image。
- 其他错误：返回 Internal。

## 路由与注册
- 新处理器统一在 ProcessorRegistry::default_registry 中注册。
- 前端通过 list_processors 获取功能清单，避免硬编码。

## 测试要求
每个处理器至少包含：
1. 参数校验测试。
2. 最小输入输出成功测试。
3. 边界测试（空图、损坏图、极端参数）。
4. 批处理失败隔离测试。
