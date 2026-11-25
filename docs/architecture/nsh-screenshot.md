# NSH 截图套件（S1–S3）

日期：2026-11-25

## 结论

- S1 框选/标注/保存/贴图/复制通过；真实鼠标 1000×600 与跨屏 600×600 均按物理像素输出。
- S2 OCR 中英、整图/区域、复制通过。长截图对 Chromium 页面受限制，与旧 Tauri 版一致；NSH 在 600px 视口可连续拼接，略好于旧版。
- S3 混合 DPI 通过（PMv2=2，125%/150% 跨屏 600×600 精确）；HDR 当前未启用，HDR/10-bit 路径未实测，仍为发布阻塞项。

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
| HDR/10-bit | 未实测 | 当前非 HDR，airsnip HDR 测试走“当前非 HDR 环境，走 BitBlt” |
| SDR 屏对比 | SDR 路径通过 | S1/S2/S3 截图均来自 SDR BitBlt/PrintWindow |

HDR 仍为发布阻塞：本机未启用 HDR，不能声称 HDR 不发灰/不过曝。需要 HDR 屏幕或明确豁免后才能关闭。

## 命令

```powershell
cargo test --manifest-path nsh/Cargo.toml --lib
cargo test --manifest-path crates/airsnip/Cargo.toml longshot --lib
cargo build --manifest-path nsh/host/Cargo.toml
```

## 证据

docs/assets/nsh-screenshot/：S1 样本、S2 OCR/剪贴板/鼠标尺寸、s2-chrome-blocker.md、old-tauri-longshot-855.png、old-tauri-longshot-600.png、s3-mixed-dpi-600x600.png。

## 发布状态

截图仍是首版发布阻塞项：Chromium 长截图限制与旧版一致，HDR/10-bit 未实测；其余 S1/S2/S3 可测项已通过。