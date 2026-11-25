# Chrome 长截图诊断原始输出（2026-11-25）

环境：Edge 15176，主窗 4132076，测试页 longshot-page.html，视口 855px。

## 诊断 1：隐藏我们所有窗口后，中心点命中谁

```
WindowFromPoint: hwnd=6229264 pid=msedge class=Chrome_RenderWidgetHostHWND
Root: hwnd=4132076 pid=msedge class=Chrome_WidgetWin_1
child level 0: hwnd=6229264 pid=msedge class=Chrome_RenderWidgetHostHWND
Chrome rect: 80,80 1100x1000
DWMWA_CLOAKED root=0 hr=0
DWMWA_CLOAKED hit=0 hr=0
GUIThread active=4132076 focus=4132076 tid=26112
Foreground=4132076
```

结论：视口中心不是我们的窗口；命中的是 Chrome 的 render 子窗，顶层为 Chrome_WidgetWin_1。Chrome 未被 cloaked，前台/Focus 都是 Chrome。

## 滚轮实际是否落到 Chrome

先真实滚轮把页面回到顶部（PrintWindow 取 x=30,y=200 的颜色）：

```
start=37,71,113
after root post=222,170,166
after child post=222,170,166
```

PostMessage WM_MOUSEWHEEL 到顶层 4132076 和 render 子窗 6229264 都能改变页面内容。posted wheel 本身有效。

## 但长截图仍只拿到首屏

- 长截图窗口可见时：自动/手动滚动后，输出始终 1100x855 首屏。
- 从外部把我们所有 airsnip.longshot* 窗口 SW_HIDE，再滚动 Chrome，长截图会话仍输出首屏（329KB，1100x855）。
- 600px 视口时自动滚动可拼出连续 0..49 行；855px 视口失败。
- PrintWindow/窗口 DC/BitBlt 都试过；屏幕抓取与 PrintWindow 在滚动后都拿到旧帧。

## 当前判断

滚动输入到 Chrome 是可行的，中心命中不是我们的窗口，Chrome 未 cloaked。问题更可能在长截图会话抓帧源/定时抓帧：窗口存在或隐藏时都拿不到 Chrome 的新合成帧，或隐藏后定时器没有继续。