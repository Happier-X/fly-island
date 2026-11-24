# 媒体与本地工具：Tauri → NSH 对照

覆盖本轮顺序：媒体播放控制、音量、设备电量、翻译、快捷启动器、文件中转，以及 Cindy 通知提示音的 parity 结论。状态以 2026-11-24 的工作区为准。

## 媒体播放控制与音量

| 能力 | Tauri 现有行为 | NSH 实现 | 取舍 |
|---|---|---|---|
| 数据源 | Windows SMTC（GlobalSystemMediaTransportControls） | 同一 SMTC，nsh/src/media | 保留 |
| 刷新 | SMTC 事件 watcher + 页面轮询 | 1.5 秒轮询（无事件 watcher） | 新增轮询，简化生命周期 |
| 元数据 | 标题、歌手、专辑、封面、插件曲目 id | 标题、歌手、播放状态 | 封面/专辑/曲目 id 暂不显示 |
| 播放控制 | 播放/暂停、上一首/下一首、跳转、随机、循环 | 播放/暂停、上一首/下一首 | 跳转 API 已移植，UI 暂未放；随机/循环暂不做 |
| 进度 | 位置、总时长、可跳转、可上/下一首 | 状态里保留位置/时长/能力字段 | UI 只显示标题与按钮 |
| 音量 | 系统默认输出主音量；另有输出峰值 | 读/写主音量，±10% 按钮 | 保留；峰值表暂不显示 |
| 多会话 | 会话列表，可指定 target | 只控制当前/第一个可用会话 | 多会话选择暂不做 |
| 音频电平 | get_audio_level 峰值 | API 已移植，UI 未用 | 暂不做可视化 |
| UI | 展开岛音乐页、歌词、封面、进度条 | 展开岛一行紧凑播放器 | 新增紧凑行，删除独立音乐页 |

保留：SMTC 读取、播放/暂停、上一首/下一首、标题/歌手、系统音量。
删除/暂缓：封面、歌词、专辑、进度条拖动、随机/循环、会话选择、音频峰值。
新增：在剪贴板面板上方的一行媒体条，无独立音乐页。

## 设备电量

| 能力 | Tauri 现有行为 | NSH 实现 | 取舍 |
|---|---|---|---|
| BLE | 读取 Battery Service 0x180F / 0x2A19 | 同一 GATT 读取 | 保留 |
| HID | 标准 Battery Strength（0x06/0x20） | 同一 HID value cap 读取 | 保留 |
| HID++ | Logitech 接收器 Battery Status / Unified Battery | 同一实现 | 保留 |
| 去重/排序 | 按类型、名称排序，按 id 与名称去重，最多 16 条 | 同一 normalize | 保留 |
| 范围 | 不读笔记本自身电池 | 同左 | 保留 |
| 刷新 | 打开电量页时读取 | 启动时 + 每 60 秒 | 新增轮询 |
| UI | 独立外设电量页，带图标/充电状态 | 展开岛一行，最多 4 个设备名与百分比 | 新增紧凑行，暂不做独立页 |
| 充电 | 数据里有 charging | 数据保留，UI 暂不显示 | 暂缓 |

## 翻译（已完成，2026-11-24）

独立翻译窗口（480×640、无边框、透明、可调整、始终置顶、跳过任务栏），展开岛“翻译”页签和全局快捷键 Ctrl+Shift+F 打开；岛内页与窗口共用状态。

- 方向：auto / zh-en / en-zh，auto 按中文与拉丁词比例判断。
- 输入：打开时从剪贴板填充空输入，有粘贴/清空；5000 字符上限；可开启停止输入 500ms 自动翻译（默认关）。
- 结果：复制、已复制状态；错误、缺 Key、超时都有提示，重试复用提交。
- 快捷键：设置里点“录制”后按组合键写入，Esc 取消，Ctrl+Shift+I/O/M/T 冲突提示，支持重置。
- Provider：DeepSeek、火山方舟、通义千问、Kimi、硅基流动、OpenAI、自定义 OpenAI 兼容；base_url/model 规则照 Tauri。
- 密钥：Windows 凭据管理器，target 与旧 keyring 条目一致，旧 Key 可直接读取。
- 旧设置：从旧 WebView2 LevelDB 只读副本导入 provider/model/方向/快捷键/开关。

完整对照表、mock 与 UI 验收证据见 [nsh-translation.md](nsh-translation.md)。

## 快捷启动器（已完成，2026-11-24）

680×460 无边框透明置顶窗口，Alt+Space 或双击 Ctrl 唤出，鼠标所在显示器居中；Esc、失焦、再按快捷键关闭。

- 索引：开始菜单 `.lnk/.url/.appref-ms`（4 层）、`Get-StartApps` 的 UWP、PATH 可执行文件；默认 Desktop/Documents/Downloads 或自定义目录（3 层、3000 条、30 秒缓存）。
- 搜索：exact/prefix/word/contains/path 排序，应用优先，短名优先；URL 直达，无结果按 Bing/Google/Baidu/DuckDuckGo 回退。
- 交互：↑↓ 循环、Enter 打开、Esc 关闭、鼠标悬停选中/单击打开；空查询、索引中、无结果、打开失败都有状态位。
- 设置：启用、组合键/双击 Ctrl、录制与重置、冲突提示、搜索项、搜索引擎、目录；旧 LevelDB 设置一次性导入。
- 打开：`ShellExecuteW`，只允许绝对路径、http(s)、`shell:AppsFolder\`；失败显示在底部。

完整对照表、验收截图与已知差异见 [nsh-launcher.md](nsh-launcher.md)。

## 文件中转（对照表已列，待实现）

| 能力 | Tauri 现有行为 | NSH 计划 | 取舍 |
|---|---|---|---|
| 添加入口 | 岛内拖放、文件选择、CLI 参数 | 宿主 WM_DROPFILES + 文件选择；CLI 后补 | 分批 |
| 存储 | JSON 持久化，最多 100 条，可按天数清理 | 同左 | 保留 |
| 条目 | 名称、路径、大小、目录标记、置顶、缺失/不可达 | 同左 | 保留 |
| 操作 | 打开、在资源管理器显示、复制文件、拖出 | 打开/显示/复制先做；拖出需 OLE | 分批 |
| 预览 | 图片缩略图、Shell 图标 | 同左 | 保留 |
| UI | 独立中转面板 + 折叠条 | 展开岛文件区 + 面板 | 保留 |

状态：本轮只完成对照表；实现排在启动器之后。

## 通知提示音 parity

Tauri 有提示音：src/features/cindy-notifications/sounds/。
- 设置只有开关（focusd-island-agent-sounds.enabled，默认开），没有音量设置。
- 三类 WAV：complete / question / error；四类事件映射为完成、提问、出错，定时结果复用完成。
- 同时到达多条时按 error > question > done > scheduled 只播一条；音量固定 0.7。
- 游戏模式或通知总开关关闭时不播。

提示音已实现：nsh/assets/sounds/thumb-piano 下三份 WAV（来自 src/assets/sounds，原创、MIT），加载时把 16-bit PCM 乘 0.7，用 winmm PlaySoundW(SND_MEMORY|SND_ASYNC|SND_NODEFAULT) 播放；设置页增加 sound_enabled，默认开；新事件按 error > question > done/scheduled 只播一条。

## 本轮实现状态

- 已完成：媒体播放控制（核心）、系统音量、设备电量（BLE/HID/HID++）。
- 已完成：翻译、快捷启动器。待实现：文件中转。
- 已做：Cindy 通知提示音（三份 WAV、0.7 音量、设置开关）。
- 媒体 parity 已完成（封面/取色、歌词、进度点击/拖动、随机循环、多会话/来源优先级、空闲态、电平字形、提示音）；验收证据见 [nsh-media-parity.md](nsh-media-parity.md)。
