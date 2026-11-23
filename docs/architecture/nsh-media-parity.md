# 媒体 parity 清单（2026-11-24）

| 项目 | Tauri 现版行为 | NSH 现状 | 本轮 |
|---|---|---|---|
| 封面 | SMTC 封面转 data URL，音乐页/折叠显示 | 媒体行显示 30px 封面 | 已做 |
| 封面取色 | artworkTint 取封面平均色驱动进度条/均衡器 | media/mod.rs 解码封面求平均色，驱动进度/随机循环/电平 | 已做 |
| 折叠歌词 | CollapsedLyric 在收起时滚动 LRC | 收起行显示当前歌词（无歌词回退歌名），展开页显示上一行/当前行/翻译/下一行；本地 LRC、LRCLIB、酷狗 KRC、网易云；无歌词/断网/源关闭/缓存失效都有状态 | 已做（2026-11-24 验收） |
| 进度 | 位置/总时长、进度条、拖动定位 | 自绘进度条：点击/拖动、拖动中冻结、松手 seek、0/末尾夹紧，时间跟随拖动位置 | 已做 |
| 随机/循环 | shuffle/repeat 按钮与状态 | 🔀/🔁 按钮，状态着色 | 已做 |
| 多会话 | 会话列表，可切 target | 设置 → 播放器优先级列出所有会话；按顺序选首个 playing，其次首个打开 | 已做 |
| 来源优先级 | pickMediaSource/useMusicSourcePriority 按设置选会话 | settings.json 的 music_sources 复刻该顺序；新播放器追加，上下箭头调整；双播放器实测 | 已做 |
| 空闲态 | MusicIdle 视觉 | 无会话时显示唱片图标、当前没有播放、提示和播放按钮 | 已做 |
| 均衡器/音频电平 | 音频峰值驱动均衡器 | 媒体行用 ▁▃▅▇ 显示 audio_peak | 已做（字形） |
| 提示音 | 3 个 WAV，0.7 音量，开关默认开 | 已复制 WAV、PCM 乘 0.7、PlaySoundW 异步播放、设置开关 | 已做 |

## 提示音实现

- 资源：nsh/assets/sounds/thumb-piano/{complete,question,error}.wav，来自 src/assets/sounds/thumb-piano，README 记录为 2026-11-06 为 fly-island 原创合成，随项目 MIT 发布。
- 播放：winmm PlaySoundW(SND_MEMORY | SND_ASYNC | SND_NODEFAULT)；加载时把 16-bit PCM 样本乘 0.7，不改系统音量。
- 触发：AI 新事件按 error > question > done/scheduled 只播一条；首次加载不补播旧事件；游戏/通知关闭不播。
- 设置：settings.json 的 sound_enabled，默认开；旧 focusd-island-agent-sounds 只迁移 enabled 的语义（本机没有该键）。

## 歌词实现

- 服务：nsh/src/lyrics 移植 local、lrclib、netease、kugou、krc、matching、found、offset；简繁匹配用 nsh/src/lyrics/chinese.rs 的 LCMapStringEx 实现，网络请求走 reqwest(rustls)，代理规则从 updater 的 get_updater_proxy 等价移植。
- 缓存：%APPDATA%\com.focusd.island.nsh\lyrics-cache\<FNV-1a>.json，条目格式与 Tauri 相同（key、lrc、source、translation、wordTimes、durationMs、durations、fetchedAt）；最近 7 天的未命中直接复用，旧格式或损坏条目会重新获取并在界面提示缓存已失效。最多 2000 个文件。
- 每首歌的偏移：%APPDATA%\com.focusd.island.nsh\lyrics-offsets.json，±10 秒、按 0.1 秒取整、最多 500 首；展开页有 -0.5s / +0.5s / 复位。
- UI：收起胶囊显示当前歌词，没有歌词或尚未取到时回退歌名；展开页显示上一行、当前行、翻译（没有翻译时显示下一行）。设置新增本地 LRC 文件夹、在线歌词（LRCLIB）、中文源（酷狗/网易云）开关，旧设置里的 lyricsFolder/lyricsOnline/lyricsChinese 会导入。
- 状态：加载中显示“正在查找歌词”；全源无结果显示“暂无歌词”；请求全部失败显示“歌词获取失败”并附错误；在线源关闭或本地目录不可用显示对应提示；中文源关闭且歌名含中文时提示打开中文源；缓存失效/过期/不可写有单独提示。
- 验证：cargo test --lib --offline（47 通过 / 5 个网络测试忽略）；cargo check --manifest-path nsh/host/Cargo.toml；cargo build --manifest-path nsh/host/Cargo.toml；宿主启动后进程 Responding=true。
- 网络实测（cargo test -- --ignored）：LRCLIB 找到 Yesterday；酷狗与网易云找到余香；酷狗 KRC 返回带逐字时间的中文歌词，Yesterday 的 KRC 返回翻译；HTTPS_PROXY 指向 127.0.0.1:9 时所有源失败并返回错误。
- 待人工验收：本机当前没有媒体会话，收起/展开歌词的真实曲目目视验收尚未做；用一首本地 LRC 或在线歌曲播放，确认收起行、展开三行、翻译行和偏移按钮的视觉与滚动节奏。

## 2026-11-24 媒体收尾验收

- 真实会话：用临时 MP3 + WinRT Windows.Media.Playback.MediaPlayer 起 SMTC 会话（标题、歌手、专辑、时长真实），再用第二个进程副本作为第二播放器。未用 mock。
- 歌词截图：本地 LRC local-*.png（《晴天》，收起行显示“我怎么看不见”，展开页显示上一行/当前行/下一行）；LRCLIB lrclib-*.png（Yesterday）；酷狗 KRC + 翻译 krc-*.png（Lemon，来源 kugou，翻译“你仍是我的光芒”）；无歌词 no-lyrics-*.png；断网 network-error-*.png（LRCLIB 不可达，显示“歌词获取失败”及错误详情）。
- 拖动：drag-start.png 时间回到 0:0x，drag-end.png 夹紧到 2:05；drag-hold-1.png 与 drag-hold-2.png 都停在 1:15（拖动时暂停跟随），drag-release.png 松手后变为 1:17（已 seek 并继续播放）。
- 多会话：multi-session-expanded.png 选中 Player2 的 Lemon；settings-sources.png 列出 Player2 与 Powershell 两行。反转 music_sources 顺序并重启后，同一对播放器从 Powershell/晴天切到 Player2/Lemon。
- 空闲态：idle-collapsed.png、idle-expanded.png，无媒体会话时显示“当前没有播放”及播放按钮，不再整行消失。
- 命令：cargo test --lib --offline（49 通过 / 5 个网络测试忽略）；cargo check --manifest-path nsh/host/Cargo.toml；cargo build --manifest-path nsh/host/Cargo.toml。
- 验收环境：本机验收时桌面处于锁屏（LockScreenBackstopFrame），因此截图用 PrintWindow，鼠标拖动通过向 NshWindow 发送鼠标消息完成；交互结果由进度时间、seek 后的 position_ms 与 OCR 共同确认。人工目视仍建议打开 docs/assets/nsh-media/2026-11-24/ 复核一次样式。

## 未完成原因

剩余：无功能项。均衡器仍是 ▁▃▅▇ 字形近似；逐字高亮动画与面板内一键开启中文源列为后续打磨。
