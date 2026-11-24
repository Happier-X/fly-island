# 微信 / QQ / 邮件：探测、对照与联调清单

## 本机探测（2026-11-23，只读）

- 微信：运行中。Weixin 主进程 pid 20376，窗口标题 Weixin/微信，类名 mmui::MainWindow，rect 2936,141,1328,963；另有 WeChatAppEx 辅助进程。未读取聊天内容，未改客户端设置。
- QQ：未运行，没有 QQ 进程或窗口。
- 邮箱：旧 LevelDB 没有 focusd-island-qq-mail-settings；cmdkey /list 没有 com.focusd.island.qq-mail 凭据，判定尚未配置。

## 微信任务栏关注提醒

| 能力 | Tauri 现有行为 | NSH 实现 | 状态 |
|---|---|---|---|
| 信号来源 | RegisterShellHookWindow，监听 HSHELL_FLASH | 宿主监听同一 SHELLHOOK，按窗口进程名 Weixin/WeChat 过滤 | 已完成 |
| 状态 | disabled / waiting / listening / error，pending + lastAttentionAt | disabled / 监听中 / 需关注 / 微信未运行 | 已完成 |
| 内容 | 只拿到“有提醒”，不读消息 | 同左；只匹配窗口/进程 | 已完成 |
| 点击 | 把微信窗口带到前台 | 宿主 SetForegroundWindow，失败保留 pending | 已完成（锁屏下无法验证聚焦） |
| 开关 | focusd-island-wechat-reminders-enabled，默认开 | settings.json 的 wechat_enabled，默认开 | 已完成 |
| 失败态 | 安装/监听错误显示在状态里 | 未运行显示“微信未运行”，监听失败写日志 | 已完成 |

验证：启动宿主后显示“监听中”；向宿主发送 SHELLHOOK/HSHELL_FLASH 且目标是真实微信窗口，状态变为“需关注”；微信当前确实在后台闪烁，未读内容。锁屏下 SetForegroundWindow 失败，pending 保留；解锁后点击“打开微信”才清除。该项列为待人工验收，不绕开系统限制。

## QQ 通知（夹具管线完成，2026-11-24）

| 能力 | Tauri 现有行为 | NSH 实现 | 状态 |
|---|---|---|---|
| 来源 | Windows UserNotificationListener 读 QQ toast | 夹具源 `NSH_QQ_FIXTURE=ok / denied / required / error`；真实监听待接 | 夹具完成 |
| 存储 | 只留内存，不落盘 | `nsh/src/qq.rs` 的 `Buffer`，最多 20 条，内存 | 完成 |
| 去重/合并 | 按条 id 去重，更新替换 | 同 id 替换；同标题 5 秒内合并正文；上限 20 | 完成 |
| 过滤 | 只识别 QQ 应用 | 真实源按 `QQ` AppId 过滤（待接）；夹具按脚本 | 夹具 |
| 点击 | 打开 QQ | 先找 QQ 窗口再尝试启动；未运行/失败返回明确错误 | 代码已接，真实联调待 |
| 失败态 | 无权限/未运行明确提示 | 未运行、权限不足、需要授权、读取失败四态 + 重试 | 完成 |
| UI | 设置开关 + 消息面板 | 设置内“QQ 通知”开关；展开岛 QQ 面板含状态、列表、打开/重试/清空 | 完成 |

夹具验收：`NSH_QQ_FIXTURE=ok` 第一次读是基线，第二次读出现“设计群”并把 5 秒内的“后续消息”合并成一条；`denied` 显示“通知权限被拒绝”，`error` 显示“读取失败”，两者都带重试。真实监听等 QQ 登录后联调。

## 邮箱（设置与夹具 IMAP 完成，2026-11-24）

| 能力 | Tauri 现有行为 | NSH 实现 | 状态 |
|---|---|---|---|
| 凭据 | Windows 凭据管理器，service=com.focusd.island.qq-mail | `CredRead/CredWrite/CredDelete`，target 为 email + service，与旧条目一致 | 完成 |
| 账户 | 最多 5 个，邮箱地址 + 可选 IMAP host | `settings.json` 的 `mail_accounts`；旧 LevelDB 的 `focusd-island-qq-mail-settings` 一次性导入 | 完成 |
| 入口 | 设置页账户表单 | 设置页“消息 → 邮箱”：服务商预设、邮箱、IMAP 服务器、授权码、保存/删除/重试/同步 | 完成 |
| 同步 | IMAP 只拉邮件头 | `mail::Source` 把 `headers()` 与 `body(id)` 分开；夹具测试确认先拉头不碰正文 | 夹具完成 |
| 正文 | 按需读取 | 点击“读取正文”才调用一次 `body(id)`；结果转纯文本 | 完成 |
| 已读 | 不标已读 | 没有 STORE/SEEN 操作 | 设计约束 |
| 附件 | 不下载 | 正文在 `--attachment--` 处截断，不解析附件 | 完成 |
| 远程图片 | 不加载 | 正文清理丢弃 `<img>`、`<script>`、`<style>`，不发起图片请求 | 完成 |
| 失败态 | 未配置/认证/超时/重试 | Unconfigured / Saved / Ready / AuthFailed / Timeout / Offline / Error，错误原因可见并可重试 | 完成 |
| 真实 IMAP | TLS IMAP | 未接入；默认显示“真实 IMAP 尚未接入”，只有夹具 env 走假服务 | 待联调 |

夹具验收：`NSH_MAIL_FIXTURE=auth` / `timeout` 分别显示“认证失败”“连接超时”；`ok` 显示两封邮件头，点击“读取正文”才出现正文，正文里的远程图片、脚本和附件段都被丢弃。测试账号为临时写入 `settings.json`，截图后已清空。

## 实现位置

- `nsh/src/qq.rs`：Notification、Buffer、Source、Fixture、Service、失败态与测试。
- `nsh/src/qq_ui.rs`：展开岛 QQ 面板（状态、列表、打开/重试/清空）。
- `nsh/src/mail/mod.rs`：服务商预设、地址/服务器校验、Header/Snapshot、Source、Fixture、正文清理与测试。
- `nsh/src/mail/credentials.rs`：`com.focusd.island.qq-mail` 的 CredRead/CredWrite/CredDelete，不打印授权码。
- `nsh/src/mail_ui.rs`：设置页账户表单、状态、账户列表、邮件头列表、按需正文。
- `nsh/src/preferences.rs`：`qq_enabled`、`mail_accounts` 与旧 LevelDB 导入。

## 自动化与截图

- `cargo test --lib`：92 passed / 10 ignored。覆盖 QQ 基线/去重/合并/上限/夹具失败态，邮件地址与预设、只拉头、正文清理、认证/超时/离线/重试、账户上限。
- 截图目录：`docs/assets/nsh-messaging/2026-11-24/`。
  - `qq-ok.png`：QQ 面板“监听中”，设计群消息与合并后的后续消息。
  - `qq-denied.png`：权限被拒绝 + 错误原因 + 重试。
  - `qq-error.png`：读取失败 + 错误原因 + 重试。
  - `mail-unconfigured.png`：邮箱未配置、服务商预设与授权码入口。
  - `mail-auth-failed.png`：认证失败与重试。
  - `mail-timeout.png`：连接超时与重试。
- 截图用夹具 env 启动，不连接真实 QQ/IMAP；临时邮箱账户已从设置中删除。


## 联调清单（等客户端登录/凭据配置后执行）

1. 微信：真实消息到达，任务栏闪烁 → 岛显示“需关注”；点击“打开微信”前台为微信且状态回到“监听中”。
2. 微信：在微信内切到会话 → 岛自动清除 pending；关闭微信 → 显示“微信未运行”。
3. QQ：登录 QQ，收到一条私聊/群聊 toast → 岛出现一条内存通知；重启岛后列表为空；点击打开 QQ。
4. QQ：关闭 QQ → 通知区显示“QQ 未运行”，不崩溃、不空白。
5. 邮箱：配置一个测试邮箱授权码 → 只出现邮件头；点开一封才读正文；服务器端未读状态不变；不下载附件、不加载远程图片。
6. 邮箱：错误授权码 → 明确“认证失败”并可重试；断网 → 超时后可重试；删除凭据 → 回到“未配置”。

## 实现状态

- 已完成：微信任务栏关注提醒（宿主 SHELLHOOK + 岛内状态/开关/打开微信）。
- 已完成：QQ 内存通知管线与夹具源（去重/合并/上限/失败态/重试）；邮箱服务商预设、Win32 凭据、账户设置与夹具 IMAP（只拉头、正文按需、不标已读/不下载附件/不加载远程图片）。
- 待联调：真实 QQ UserNotificationListener 与 QQ 窗口唤回；真实 TLS IMAP 连接与授权码验证。
