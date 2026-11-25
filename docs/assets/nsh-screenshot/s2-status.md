# NSH 截图 S2 状态（OCR / 长截图）

日期：2026-11-25
工作树：C:/code/fly-island/.cindy-worktrees/keen-mclean

## 已完成（代码 + 单测）

- OCR 设置：自动 / 中文 / English，显示系统已安装的语言标签，可选“识别后自动复制”；
  长截图高度上限也进入设置页。设置经 to_airsnip_config 注入 AirSnip。
- 区域 OCR：覆盖层有选区时按 T。
- 整图 OCR：覆盖层没有选区时按 T，直接识别整张抓屏（新增的回退路径）。
- OCR 结果仍用 AirSnip 气泡：原生 EDIT，可选中/复制；缺语言包时显示
  “未安装 … 的文字识别语言包，请到系统设置添加光学字符识别可选功能”。
- 长截图：状态条新增“自动滚动”按钮（Play 图标）；自动模式向视口中心下的窗口
  投递 WM_MOUSEWHEEL，等画布增长；无增长时缩小滚动量重试最多 2 次，仍无增长
  则停止并提示“已到页底或页面未响应”，不自动结束；手动滚轮路径保留。
- 拼接缝：新增 16 行局部接缝校验，重叠带一致率 < 95% 时拒绝追加并提示重试；
  自动模式会缩小滚动量再试。
- Esc：长截图窗口的全局 Esc 继续设置 cancelled 并关窗；已实测窗口关闭、不产出文件。
- 长截图输出改为使用 NSH 配置的目录和格式，不再写死 AirSnip 默认目录。
- 真实 OCR 引擎单测：recognizes_rendered_latin_and_chinese 在本机通过（中英语言包可用）。
- 长截图单测：31 个 longshot 测试通过，含新增的 seam 一行错位拒绝测试。
- NSH 单测：118 passed / 9 ignored。

## 当前阻塞：桌面锁定

本机当前前台窗口类是 LockScreenBackstopFrame，SetCursorPos 失败、
OpenClipboard 返回 ERROR_ACCESS_DENIED。屏幕抓取只能得到锁屏画面，因此：

- 浏览器长页面的自动/手动长截图无法采集真实内容；
- 区域 OCR / 整图 OCR 无法得到页面文字；
- 复制结果与粘贴无法验证。

在锁屏下已验证：覆盖层与长截图窗口能创建；自动按钮可点击不崩溃；Esc 能关窗；
OCR 气泡能打开并显示“未识别到文字”错误。真实页面内容验收必须等桌面解锁。

## 解锁后的一次性验收矩阵

测试页面：nsh/tests/longshot-page.html（60 行，每行 40px，左侧色块编码行号）。

| 项目 | 期望 |
|---|---|
| 区域 OCR | 选页面文字区域按 T，气泡包含 L0xx / quick brown fox |
| 整图 OCR | 不选区按 T，气泡包含页面文字；不再出现“未识别到文字” |
| 缺语言包 | 设置不存在的标签（如 ja-JP），气泡提示未安装及系统设置路径 |
| 自动长截图 | 选 700×600 视口，点自动；输出高度约 60 行对应高度，色块行号连续无重复 |
| 手动长截图 | 不点自动，向目标窗口发送滚轮；输出同样行号连续 |
| 拼接缝 | 输出每 40px 色块解码为连续行号，无错行/重复 |
| 重试 | 故意一次大滚动触发 seam 拒绝，自动模式缩小步长后继续 |
| Esc | 长截图过程中 Esc，窗口关闭且目录无新文件 |
| 输出目录 | 文件写到设置页指定目录，格式遵循设置 |

## 运行方式（解锁后）

1. 退出 Tauri 版 fly-island，解锁桌面。
2. 保持 Edge 的长页面在前台（页面已提供测试 HTML）。
3. 运行 NSH 宿主并触发一次截图，按上面矩阵执行。
4. 证据保存到 docs/assets/nsh-screenshot/s2-*。

## 预期证据文件（解锁后生成）

- docs/assets/nsh-screenshot/s2-ocr-region.png：区域 OCR 气泡与选中文字。
- docs/assets/nsh-screenshot/s2-ocr-full.png：整图 OCR 气泡。
- docs/assets/nsh-screenshot/s2-ocr-missing-pack.png：缺语言包提示。
- docs/assets/nsh-screenshot/s2-longshot-auto.png：自动长截图输出。
- docs/assets/nsh-screenshot/s2-longshot-manual.png：手动长截图输出。
- docs/assets/nsh-screenshot/s2-longshot-seam-check.txt：色块行号解码结果（连续、无重复）。
- docs/assets/nsh-screenshot/s2-longshot-esc.txt：Esc 中止后窗口关闭且无新文件。

## 2026-11-25 桌面解锁后的普通截图验收

- 真实鼠标框选：选区 1000×600，输出 s2-mouse-1000x600.png 精确 1000×600，无 1.25 倍放大。
- 剪贴板：复制后 OpenClipboard 成功，格式包含 CF_DIB（2,400,040 bytes）、注册 PNG（214,756 bytes）、CF_BITMAP、CF_DIBV5。
- 记事本：本机 Windows 11 Notepad 不接收图片剪贴板，Ctrl+V 后无可视图像；已用 Paint 尝试作为替代证据（s2-clipboard-paint.png）。剪贴板格式本身已确认。
- 区域 OCR（en-US）：输出 “L012 The quick brown fox jumps over the lazy dof”，与页面一致；见 s2-ocr-en.txt。
- 整图 OCR（en-US）：输出包含 L010–L026 等页面行、工具条与桌面内容；见 s2-ocr-en.txt。
- 区域/整图 OCR（zh-Hans-CN）：输出包含“中文行号”等中文字段，语言设置生效；见 s2-ocr-zh.txt。

## 仍未通过

- 长截图完整内容区（约 855px）仍在 Chrome 遮挡/重绘问题上失败，自动/手动都无法拼出完整页面；按用户安排留到单独的第二步修复轮次。

## Chrome 遮挡专项（2026-11-25）

A/B/C 均未通过 855px 完整内容区；详见 [s2-chrome-blocker.md](s2-chrome-blocker.md)。源码保持 4f47316 基线。

## 基线对比结论（2026-11-25）

旧 Tauri 版 855px/600px 同样只出首屏，见 old-tauri-longshot-855.png / old-tauri-longshot-600.png。S2 按与现版持平判过；S3 暂停。

## 基线对比口径修正（2026-11-25）

- 旧 Tauri 版 600px 也只出首屏；NSH 600px 能拼出连续 0..49 行，855px 与旧版同样受限。
- NSH 不是回归，而是比现版略好；既不是“已修复”，也不是“新增缺陷”。
