# 媒体 parity 清单（2026-11-23）

| 项目 | Tauri 现版行为 | NSH 现状 | 本轮 |
|---|---|---|---|
| 封面 | SMTC 封面转 data URL，音乐页/折叠显示 | MediaState 已有 thumbnail 字段，UI 未显示 | 不做；需补封面布局与缓存 |
| 封面取色 | artworkTint 取封面平均色驱动进度条/均衡器 | 未计算 | 不做；随封面一起补 |
| 折叠歌词 | CollapsedLyric 在收起时滚动 LRC | 未实现歌词服务/LRC 解析/滚动 | 不做；需本地/LRCLIB 歌词源 |
| 进度 | 位置/总时长、进度条、拖动定位 | 状态里有 positionMs/durationMs，UI 未显示 | 不做；NSH 暂无 range/拖动控件，需自绘命中 |
| 随机/循环 | shuffle/repeat 按钮与状态 | 协议已移植，UI 未放 | 不做；待进度条一起补 |
| 多会话 | 会话列表，可切 target | 状态里有 sessions，UI 只控制当前 | 不做；需来源选择 UI |
| 来源优先级 | pickMediaSource/useMusicSourcePriority 按设置选会话 | 未实现 | 不做；需设置项与选择逻辑 |
| 空闲态 | MusicIdle 视觉 | 无会话时整行隐藏 | 不做；按用户允许暂时隐藏 |
| 均衡器/音频电平 | 音频峰值驱动均衡器 | audio_level API 已移植，UI 未用 | 不做；随空闲态一起补 |
| 提示音 | 3 个 WAV，0.7 音量，开关默认开 | 已复制 WAV、PCM 乘 0.7、PlaySoundW 异步播放、设置开关 | 已做 |

## 提示音实现

- 资源：nsh/assets/sounds/thumb-piano/{complete,question,error}.wav，来自 src/assets/sounds/thumb-piano，README 记录为 2026-11-06 为 fly-island 原创合成，随项目 MIT 发布。
- 播放：winmm PlaySoundW(SND_MEMORY | SND_ASYNC | SND_NODEFAULT)；加载时把 16-bit PCM 样本乘 0.7，不改系统音量。
- 触发：AI 新事件按 error > question > done/scheduled 只播一条；首次加载不补播旧事件；游戏/通知关闭不播。
- 设置：settings.json 的 sound_enabled，默认开；旧 focusd-island-agent-sounds 只迁移 enabled 的语义（本机没有该键）。

## 未完成原因

NSH 目前没有 range/拖动控件与歌词服务，封面取色也需要图像解码后在 UI 侧接进度条/均衡器。这些不是单点开关，需要一轮完整的媒体页改造；本轮先完成提示音，避免把未完成项标记为完成。
