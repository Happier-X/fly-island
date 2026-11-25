# Chrome 长截图遮挡/停止出帧实测结论

日期：2026-11-25
基线：4f47316（未修改源码，实验代码已回滚）

## 实测对象

- Edge 主窗：Chrome_WidgetWin_1（longshot-page.html）
- Edge 渲染子窗：Chrome_RenderWidgetHostHWND
- 长截图窗口：airsnip.longshot（状态条）、airsnip.longshot.preview、airsnip.longshot.border
- 测试页：nsh/tests/longshot-page.html；完整内容区约 1100×855 物理像素

## A. 滚动期间隐藏/移出所有 overlay，屏幕 BitBlt

做法：
- 状态条移到屏幕底边，preview 和 border 使用 SW_HIDE；
- 自动滚动通过真实 mouse_event 滚轮注入，等待 session 高度增长；
- 抓帧使用屏幕 BitBlt（ScreenSource）。

结果：
- 自动模式确实启动并发送滚轮（日志 [auto] started / scroll at 855）；
- session 高度始终停在 855，输出只有 1100×855 首屏；
- 没有新帧进入拼接。

## B. 窗口属性避免遮挡检测

做法：
- border/preview 原本已有 WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW；
- 另外尝试移除 border、把 border 改为四条边窗口区域；
- 状态条保留可点击但移到屏幕边缘。

结果：
- Chrome 仍不产生新滚动帧；输出仍只有首屏。
- 600px 视口时自动滚动可拼出连续 0..49 行（说明滚动/拼接链路在较小遮挡面积下可用），但 855px 完整内容区始终失败。

## C. Chrome 渲染子窗 PrintWindow

做法：
- 目标窗口解析为顶层 Chrome_WidgetWin_1，并尝试打印 Chrome_RenderWidgetHostHWND；
- PrintWindow(PW_RENDERFULLCONTENT=2) 成功返回非黑帧；
- 滚动后调用 RedrawWindow(RDW_INVALIDATE|RDW_UPDATENOW|RDW_ALLCHILDREN) 再 PrintWindow；
- 也试过 GetWindowDC+BitBlt（返回黑帧，已排除）。

结果：
- PrintWindow 能拿到首帧；滚动后重复 PrintWindow 仍是同一帧；
- 帧内容不随页面 scrollTop 变化，拼接高度不增长。
- 未给 Chrome 加任何命令行参数。

## D. 结论

A/B/C 均未通过 855px 完整内容区验收。可复现事实：
- 600px 视口自动滚动可连续拼接（0..49，无重复/缺失）；
- 完整 855px 视口下 Chrome 在长截图覆盖/滚动后不再提供新帧，屏幕 BitBlt、窗口 PrintWindow 都拿到旧帧。
- 需要新的取舍：限制长截图视口高度、改变长截图交互模型（滚动时彻底移出屏幕/最小化覆盖 UI 由独立进程控制），或接受 600px 视口并裁掉页面末尾。
