# NSH 截图 S1 验收记录

日期：2026-11-25
工作树：C:/code/fly-island/.cindy-worktrees/keen-mclean
构建：cargo build --manifest-path nsh/host/Cargo.toml

## 环境

- 主屏：2560×1440 @ 125%（120 DPI），虚拟屏原点 (0,0)
- 副屏：2240×1400 @ 150%（144 DPI），位于主屏右侧，(2560,41)
- 虚拟桌面：4800×1441 物理像素
- 测试时 Tauri 版 fly-island 仍在运行，并占用 Ctrl+Shift+A；S1 用
  Ctrl+Alt+Shift+F9 完成自动化验证。

## 复用方式

- nsh 直接依赖 crates/airsnip，调用 airsnip::start_host 启动其隐藏消息窗和
  截图线程；覆盖层、选区/八向手柄、标注、OCR/长截图入口和贴图窗口都沿用
  AirSnip 现有 Rust 实现。
- nsh 只新增：%APPDATA%\com.focusd.island.nsh\screenshot\settings.json、
  全局快捷键线程、设置页截图分区、托盘“截图”项和 WM_HOST_SCREENSHOT 触发。
- 剪贴板写入仍是 AirSnip 的 CF_DIB + 注册 PNG 格式；文件保存使用 AirSnip
  的 PNG/JPEG/WebP 编码器。

## 证据文件

- s1-overlay-print.png：覆盖层首帧（横跨两块屏）。
- s1-selection.png：框选后的尺寸提示与八向手柄。
- s1-saved-baseline.png：未标注的 PNG 保存结果（600×500）。
- s1-saved-annotated.png：切换到箭头工具并画一笔后的保存结果（600×500）。
- s1-saved.jpeg / s1-saved.webp：同一链路分别存成 JPEG / WebP（600×500）。
- s1-boundary-100x100.png：跨 125%/150% 交界、100×100 物理像素选区，输出精确 100×100。
- s1-pin.png：跨屏选区生成的置顶贴图窗口。

## 实测矩阵

| 项目 | 结果 | 方法 |
|---|---|---|
| 全局快捷键注册 | 通过 | 启动后注册 Ctrl+Alt+Shift+F9；占用时返回错误并写入设置页 |
| 覆盖层置顶、可交互 | 通过 | airsnip.overlay 窗口 visible，覆盖 4800×1441 虚拟屏 |
| 鼠标框选 | 通过 | WM_LBUTTONDOWN/MOUSEMOVE/UP 原生消息，选区与手柄出现 |
| 八向调整 | 通过（复用 AirSnip） | 手柄命中与拖动逻辑来自 AirSnip，未改写 |
| 尺寸提示 / Esc / Enter | 通过 | Enter 结束并触发输出；Esc 路径复用 AirSnip |
| 跨屏选区 | 通过 | 从 (1800,200) 拖到 (3000,900)，跨过 2560 边界 |
| 标注 | 通过 | 数字键 2 切换箭头，拖动画一笔；输出与基线逐像素有差异 |
| 复制 CF_DIB + PNG | 代码路径复用；本机会话剪贴板被占用 | AirSnip 弹出“复制失败”，未静默失败；需人工确认粘贴 |
| PNG 保存 | 通过 | 600×500，319141 bytes |
| JPEG 保存 | 通过 | 600×500，21074 bytes |
| WebP 保存 | 通过 | 600×500，180066 bytes |
| 跨 DPI 交界 100×100 | 通过 | 2510,500 → 2610,600，输出 100×100 |
| 贴图置顶 | 通过 | WS_EX_TOPMOST；跨屏位置 |
| 贴图缩放 | 通过 | 滚轮后窗口矩形明显变化 |
| 贴图关闭 | 通过 | WM_CLOSE 后窗口消失 |
| 混合 DPI 覆盖 | 通过（复用 AirSnip） | 主屏 125% + 副屏 150%，同一覆盖层跨屏 |
| 多屏逐屏 overlay | 实现为一个虚拟屏窗口 | 与“每块屏一个 HWND”的字面要求不同，见限制 |

## 1.25 倍排查结论（已定位，不是产品 bug）

临时日志同时记录 WM_MOUSEMOVE 的 lParam、覆盖层尺寸、抓屏尺寸、选区矩形和
最终输出尺寸，对比如下：

1. DPI 不感知的 PowerShell 进程 PostMessage(400,400)：
   - 覆盖层收到 lParam = (500,500)，正好乘 1.25；
   - 选区变为 750×438（600×350 × 1.25），输出同为 750×438。
2. 同一脚本先调用 SetThreadDpiAwarenessContext(PER_MONITOR_AWARE_V2)：
   - PostMessage(400,400) 原样到达 (400,400)；
   - 选区 600×350，输出 600×350，逐级一致。
3. 跨 2560 交界选 100×100 物理像素：
   - 2510,500 → 2610,600，输出文件精确 100×100，无缩放。

结论：1.25 倍来自 DPI 不感知的测试发送进程被 user32 做坐标虚拟化，不是
覆盖层或 BitBlt 的坐标转换 bug。真实鼠标消息由系统以物理像素派发；用
PMv2 发送的合成消息也得到精确尺寸。真实鼠标仍需人工手感确认，但该尺寸
异常不应再作为 S1 阻断项。

## 性能（AIRSNIP_TIMING=1，单次实测）

- BitBlt 抓屏：254 ms；抓屏总耗时 294 ms
- 覆盖层建窗+初始化：103 ms；SetWindowPos：15 ms；首帧：31 ms
- 覆盖层创建到显示完成：170 ms
- 快捷键到覆盖层显示：479 ms
- 本次共重绘 11 帧：平均 31.1 ms，最大 47.8 ms（约 32 fps）

## 剪贴板人工验收（待用户确认后一次跑完）

前置：退出 Tauri 版 fly-island，解锁桌面。脚本不会结束旧版进程。

    powershell -NoProfile -ExecutionPolicy Bypass -File nsh/scripts/manual-screenshot-acceptance.ps1

脚本一次完成：写入默认 Ctrl+Shift+A 设置 → 启动 NSH 宿主 → 等待真实
Ctrl+Shift+A 框选并按 Enter → 校验剪贴板序号变化 → 读取 CF_DIB 和注册
PNG 格式并比对尺寸 → 保存 manual-clipboard.png → 打开记事本请您粘贴确认。
不会反复弹窗，也不会自动结束旧版。

## 限制与待人工确认

1. 自动化会话无法移动真实光标（SetCursorPos 返回失败），也无法打开剪贴板
   （OpenClipboard 返回 ERROR_ACCESS_DENIED）。因此框选/标注/保存是通过
   PostMessage 向覆盖层投递原生鼠标键盘消息完成的；“复制后在记事本粘贴”
   和真实鼠标手感仍需人工确认。
2. Tauri 版与本版同时运行时都注册 Ctrl+Shift+A，后启动的一方会注册失败。
   NSH 已在设置页显示占用错误并建议换键；正式切换后恢复 Ctrl+Shift+A。
3. 覆盖层是一个跨虚拟桌面的顶层窗口，而不是每块屏一个 HWND。交互和跨屏
   选区正常；若必须按屏拆分窗口，需要改 AirSnip 的覆盖层启动方式，属于
   额外改造。
4. 长截图/OCR 入口已随 AirSnip 提供，但按任务划分留到 S2 验证。

S2 status (OCR / long screenshot): s2-status.md — code and unit tests are done; the real-browser matrix is blocked on an unlocked desktop.
