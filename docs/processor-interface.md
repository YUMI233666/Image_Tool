# Processor 接口规范

本文档用于约束 Art Tool 的处理器扩展接入方式，适用于图像格式转换、图像压缩、图像修复、分辨率变换等后续功能。

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
