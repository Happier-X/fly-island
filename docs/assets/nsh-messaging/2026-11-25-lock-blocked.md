# NSH 真机联调：2026-11-25 桌面被锁，交互项暂停

## 结论

桌面在联调开始时处于锁定状态，真实鼠标/键盘无法安全使用，本轮只完成不依赖解锁的部分（QQ 监听过滤、面板渲染；微信 SHELLHOOK 提醒注入与面板渲染）。其余项等解锁后按原顺序补跑。

## 锁定证据

- `WTSQuerySessionInformation(WTSSessionInfoEx)`：SessionFlags = 0（WTS_SESSIONSTATE_LOCK）。
- `GetForegroundWindow()` = 0，`GetGUIThreadInfo(0).hwndActive` = 0；桌面没有活动窗口。
- `LogonUI.exe` 在运行。
- `GetLastInputInfo` 空闲约 990 秒。
- 因此未发送任何 SendInput 鼠标/键盘事件，避免输入落到锁屏。

## 本轮已验（不依赖解锁）

| 项目 | 结果 | 证据 |
|---|---|---|
| QQ 通知监听 + AUMID 过滤 | 通过（真实 Windows 通知管道） | 用 QQ 的 AUMID（Start 菜单 AppID = `QQ`）投递一条测试 toast；监听计数 47→48，其中 QQ=1；QQ 面板出现条目 |
| QQ 面板渲染 | 通过 | 2026-11-25-uia.txt（UIA 文本） |
| 微信 SHELLHOOK 提醒 → 需关注 | 通过（注入真实微信窗口句柄 68254） | 面板从“监听中”变为“需关注 / 打开微信”；2026-11-25-uia.txt（UIA 文本） |

说明：

- 上述 QQ toast 是自动生成的测试通知，不是真实好友消息；正文为“自动生成的 QQ AUMID 测试通知，不是真实消息”。
- 微信提醒是向宿主投递 SHELLHOOK/HSHELL_FLASH（目标为真实微信主窗口），与真实消息触发的任务栏闪烁同一条链路，但不是手机端发来的真实消息。
- 锁屏下 DWM 不再更新被遮挡的窗口，`PrintWindow` 两次得到同一帧，因此没有用 PNG 作证，改用 UIA 文本。

## 未验（解锁后补）

1. QQ：点击条目唤回 QQ 窗口；通知权限未授权路径。
   - 已尝试把 `HKCU\...\CapabilityAccessManager\ConsentStore\userNotificationListener` 置为 `Deny` 并重启进程，`GetAccessStatus` 仍返回 Allowed（系统侧有缓存），未改动系统设置页；该路径仍只由单元测试覆盖。
2. 微信：真实消息、点击拉前台并清 pending、关闭 wechat_enabled 后不提示、同一条不重复提醒。
3. 文件中转：真实鼠标从 Explorer 拖入、OLE 拖出、复制路径（OpenClipboard）。
4. 截图设置页：逐项点击并确认 AirSnip 覆盖层即时生效（需要鼠标点击齿轮打开设置；锁屏下不可用）。
5. 启动器：真实键盘 ↑↓/Enter/Esc、双击 Ctrl、快捷键录制与冲突提示、目录增删。

## 当前机器状态

- NSH 宿主仍在运行（锁屏前启动），岛里保留了一条注入的“微信需关注”，点击“打开微信”或激活微信窗口即可清掉。
- 旧 Tauri 版 focusd-island（pid 16732）仍在运行并占用 Ctrl+Shift+A。
