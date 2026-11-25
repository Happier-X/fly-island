# NSH 截图套件（S1–S3）

日期：2026-11-25

## 结论

- S1 框选/标注/保存/贴图/复制通过；真实鼠标 1000×600 与跨屏 600×600 均按物理像素输出。
- S2 OCR 中英、整图/区域、复制通过。长截图对 Chromium 页面受限制，与旧 Tauri 版一致；NSH 在 600px 视口可连续拼接，略好于旧版。
- S3 混合 DPI 通过（PMv2=2，125%/150% 跨屏 600×600 精确）；HDR 已在本机临时开启验证（DXGI 240nits，PNG alpha 全 255，无整屏发灰/爆白），验后已恢复。

## 复用与结构

- NSH 以库方式承载 crates/airsnip：airsnip::start_host 起隐藏消息窗和截图线程；覆盖层、选区、标注、OCR、长截图、贴图都复用 AirSnip。
- NSH 负责设置（%APPDATA%\com.focusd.island.nsh\screenshot\settings.json）、全局快捷键、托盘入口和 WM_HOST_SCREENSHOT。
- 截图保存目录/格式走 NSH 设置；进程 PMv2。

## S1 矩阵

| 项目 | 结果 | 证据/说明 |
|---|---|---|
| 快捷键/覆盖层 | 通过 | 默认 Ctrl+Shift+A；冲突时设置页提示换键 |
| 框选/八向/尺寸提示 | 通过 | AirSnip 原生覆盖层 |
| 标注/颜色/线宽/撤销 | 通过 | 复用 AirSnip 工具集 |
| PNG/JPEG/WebP 保存 | 通过 | s1-saved-* |
| 贴图置顶/缩放/关闭 | 通过 | s1-pin.png |
| CF_DIB+PNG 复制 | 通过 | CF_DIB 2,400,040 bytes、PNG 214,756 bytes、DIBV5 |
| 真实鼠标 1000×600 | 通过 | s2-mouse-1000x600.png 精确 1000×600 |
| 记事本粘贴 | 受应用限制 | Windows 11 Notepad 不接收图片；格式本身正确 |

## S2 矩阵

| 项目 | 结果 | 证据/说明 |
|---|---|---|
| 区域 OCR en-US | 通过 | L012 The quick brown fox...，s2-ocr-en.txt |
| 整图 OCR en-US | 通过 | 包含 L010–L026 页面行，s2-ocr-en.txt |
| zh-Hans-CN OCR | 通过 | 识别“中文行号”，s2-ocr-zh.txt |
| 长截图 600px 自动 | 通过（NSH） | 连续 0..49 行；旧 Tauri 600 仅首屏 |
| 长截图 855px 完整内容区 | 与旧版一致受限 | 旧 Tauri/NSH 都仅首屏；见 s2-chrome-blocker.md |
| 拼接缝校验/重试/Esc | 已实现并单测 | seam 16 行、≥95%；Esc 关窗 |

Chromium 长截图限制：旧 Tauri 855px 输出 1100×855、600px 输出 1000×600，均仅首屏；NSH 600px 能连续拼接，855px 同旧版受限。不是 NSH 回归，也不是新增缺陷；未记为已修复。

## S3 矩阵

| 项目 | 结果 | 证据/说明 |
|---|---|---|
| 混合 DPI 跨屏选区 | 通过 | 2400,300 → 3000,900，输出 600×600；s3-mixed-dpi-600x600.png |
| 区域跨 125%/150% 边界 | 通过 | 输出尺寸与物理像素一致 |
| 进程 DPI 感知 | 通过 | GetProcessDpiAwareness=2（PMv2） |
| HDR/10-bit | 通过（本机临时开启） | DisplayConfig 支持 HDR；临时开启后 DXGI 报告 2560×1440@240nits；s3-hdr-gradient.png |
| SDR 屏对比 | SDR 路径通过 | S1/S2/S3 截图均来自 SDR BitBlt/PrintWindow |

HDR 已在本机临时开启验证：DXGI HDR 输出 2560×1440@240nits，NSH 保存的 1040×600 PNG alpha 全 255，平均亮度 HDR 228.2 / SDR 236.5，没有整屏发灰或爆白；随后已恢复 HDR 关闭。直接 hdr_capture_works 的 alpha 断言在混合 HDR/SDR 下会失败，是该测试只填 HDR output 的测试工件；完整 NSH 抓图会补 SDR 屏，保存 PNG alpha 全 255。

## 设置页对齐（旧版差异已补齐）

- 色板：5 个 #RRGGBB 输入 → Config.swatch_colors（RGBA，未填或非法值回退默认色）。
- 命名模板：日期+时间 / 时间戳 / 仅日期 → Config.name_style。
- 界面缩放：自动 / 100 / 125 / 150 / 175 / 200 → Config.ui_scale。
- 工具键：矩形、箭头、文本、模糊、高亮、聚光灯、序号、直线、椭圆、手绘共 10 项；填单个字母/数字或 F1–F24，留空为不绑定 → Config.tool_keys。
- 工具栏显隐：5 组共 25 项 → Config.toolbar.hidden。
- 覆盖：cargo test --manifest-path nsh/Cargo.toml --lib（色板解析、工具键往返、显隐过滤、枚举映射）。

## 命令

```powershell
cargo test --manifest-path nsh/Cargo.toml --lib
cargo test --manifest-path crates/airsnip/Cargo.toml longshot --lib
cargo build --manifest-path nsh/host/Cargo.toml
```

## 证据

docs/assets/nsh-screenshot/：S1 样本、S2 OCR/剪贴板/鼠标尺寸、s2-chrome-blocker.md、old-tauri-longshot-855.png、old-tauri-longshot-600.png、s3-mixed-dpi-600x600.png、s3-hdr-gradient.png、s3-sdr-gradient.png、s3-environment.txt。

## 发布状态

截图已完成首版所需的 S1–S3：HDR/混合 DPI 已通过；Chromium 长截图限制与旧版一致，不按缺陷计；NSH 设置页补齐自定义色板、命名模板、工具键、工具栏显隐与界面缩放，对丢矩阵无未分类行。截图不再阻塞发布。

## 旧 Tauri 截图能力对丢矩阵（2026-11-25）

| 能力 | 旧 Tauri 截图套件 | NSH | 结论 |
|---|---|---|---|
| 全局快捷键启用/录制/重置 | 支持 | 支持（Ctrl+Shift+A 可配置，冲突提示） | 已对齐 |
| 框选/八向手柄/尺寸提示 | 支持 | 复用 AirSnip | 已对齐 |
| 跨屏/混合 DPI | 支持 | 真实鼠标 600×600 跨 125%/150% 通过 | 已对齐 |
| 矩形/椭圆/箭头/直线/自由画笔 | 支持 | 复用 AirSnip 工具集 | 已对齐 |
| 文字/字体/字号/粗斜体 | 支持 | 复用 AirSnip | 已对齐 |
| 马赛克/模糊/高亮/聚光灯 | 支持 | 复用 AirSnip | 已对齐 |
| 序号标记 | 支持 | 复用 AirSnip | 已对齐 |
| 颜色/线宽 | 旧版设置可改 5 色板；覆盖层可双击取色 | 覆盖层一致；NSH 设置页可改 5 个色块（#RRGGBB） | 已对齐 |
| 撤销/重做 | 支持 | 复用 AirSnip | 已对齐 |
| 复制 CF_DIB+PNG | 支持 | 已实测 CF_DIB/PNG/DIBV5 | 已对齐 |
| 保存 PNG/JPEG/WebP | 支持 | 通过，目录/格式走 NSH 设置 | 已对齐 |
| 命名模板/最近格式目录 | 旧版设置可选命名模板 | NSH 设置页可选日期+时间 / 时间戳 / 仅日期 | 已对齐 |
| 贴图/钉图/拖动/缩放/关闭/锁定/置顶 | 支持 | 复用 AirSnip | 已对齐 |
| 取色器 | 支持 | 复用 AirSnip Alt 取色 | 已对齐 |
| OCR 区域/整图/语言/自动复制 | 旧版区域识别 | NSH 区域+整图、中英语言、自动复制 | 已对齐（NSH 略多） |
| 长截图手动/自动/拼接/撤销/Esc | 旧版手动；600/855 仅首屏 | NSH 手动+自动；600 可连续，855 与旧版一致受限 | 已对齐（NSH 600 略好） |
| 保存历史/截图历史列表 | 无截图历史列表，仅最近格式/目录 | 同 | 已对齐 |
| 工具快捷键/隐藏工具栏/UI 缩放 | 旧版设置可配 | NSH 设置页可配 10 个工具键、25 个工具栏显隐项、6 档界面缩放 | 已对齐 |
| HDR/10-bit | 复用 AirSnip | 本机临时开 HDR：DXGI 240nits，PNG alpha 全 255，亮度 HDR 228.2/SDR 236.5 | 已对齐 |

设置页差异已补齐：色板、命名模板、工具键、工具栏显隐、界面缩放都写入 settings.json 并注入 AirSnip Config（cargo test 覆盖解析与映射）。对丢矩阵没有未分类行。截图不再作为发布阻塞项；Chromium 长截图限制与旧版一致，按“与现版持平”记录。
