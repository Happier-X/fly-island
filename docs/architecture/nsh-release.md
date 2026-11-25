# NSH 发版与更新链路（2026-11-25）

## 打包

命令：

    nsh package nsh --target windows -o dist-nsh

产物：

- dist-nsh/FocuSD Island (NSH)/：便携目录，包含 exe、nsh/app.json、nsh/assets/。
- dist-nsh/FocuSD Island (NSH).zip：第一个可分发形态，解压即用。
- dist-nsh/FocuSD Island (NSH).iss：Inno Setup 脚本。当前机器未安装 Inno Setup，未生成安装器；需要用户自行安装 Inno Setup 6 后运行 ISCC。

便携版不写系统目录、不注册卸载项；应用数据在 %APPDATA%\com.focusd.island.nsh。

## 更新清单

端点在设置 → 自动更新中填写，留空关闭。清单是独立 JSON：

    {
      "version": "0.1.2",
      "url": "https://example.com/FocuSD%20Island%20(NSH).zip",
      "sha256": "64 位小写十六进制",
      "signature": null,
      "notes": "本次更新说明"
    }

- version 必须是严格 semver，且高于当前版本才会提示。
- url 指向便携 zip；生产环境应使用 HTTPS。本地测试允许 HTTP。
- sha256 必填，下载后强制校验；缺失或格式错误直接拒绝。
- signature 可选，只作为元数据展示；当前没有公钥配置和签名校验，不能把有该字段当成“已验证”。
- notes 可选，显示在更新设置页。

宿主启动时如果端点非空，会在后台检查；设置页也提供“检查更新”。发现新版本后显示版本、notes 和签名状态，点击“下载并重启”才执行下载。

## 下载与替换

1. 下载到 %TEMP%\focusd-island-nsh-update，限制 240 MB。
2. 计算 SHA-256，与清单严格比较。
3. 解压 zip，找到包含 nsh/app.json 的包根。
4. 覆盖目标目录：默认是当前 exe 目录；可用 NSH_UPDATE_TARGET_DIR 指向临时目录做验收。
5. 当目标是当前安装目录时，写一个 cmd 辅助脚本，等待当前进程退出后 xcopy 覆盖并启动新 exe；临时 target 会直接复制并启动新 exe。

失败状态：HTTP 404/网络错误、SHA-256 不符、zip 无效、找不到 app.json、目标不可写、启动失败，都会回到设置页的错误文本，不删除已有安装。

## 发布与安全

- 当前没有代码签名证书，便携版/zip/安装器均为未签名。Windows SmartScreen 可能显示“Windows 已保护你的电脑”，需要用户确认；这不是已验证签名。
- 强制 SHA-256 只能防止下载损坏/被替换后的内容不一致，不能证明发布者身份；谁控制端点谁就能提供新包。
- 生产应使用 HTTPS、限制清单/包访问权限，后续有证书时再补 Authenticode 和清单签名；在此之前文档和 UI 不得显示“已签名”。
- 回滚：保留上一版便携目录/zip；NSH 更新不修改 Tauri 数据，也不自动删除旧数据。

## Tauri 版本过渡

- 两版并存：Tauri 使用 %APPDATA%\com.focusd.island 和 com.focusd.island；NSH 使用 %APPDATA%\com.focusd.island.nsh 和 com.focusd.island.nsh。localStorage、LevelDB、凭据条目和更新端点互不覆盖。
- NSH 首次启动只读导入旧设置/剪贴板等白名单字段；旧文件不修改、不删除。
- 旧 Tauri 版建议在 NSH 版设置、剪贴板、消息、外观均确认后再卸载；卸载前保留旧数据目录作为回滚。
- 若 NSH 更新失败，关闭 NSH 后重新运行上一版便携目录即可；不要用 Tauri 的安装器覆盖 NSH 目录。
- 自动更新端点不要与 Tauri 的 latest.json 共用：两版格式不同，服务端需分开路径或按版本分流。
- 第一版 NSH 必须包含截图套件；不能以“截图继续留在 Tauri 旧版”作为发布口径。截图由独立专项任务实现，但其完成是首版发布前置条件。
- 截图现状（2026-11-25）：S1–S3 完成，混合 DPI/HDR 通过；Chromium 长截图与旧版一致受限（旧版 600/855 仅首屏，NSH 600 可连续拼接、855 同旧版），按“与现版持平”记录，不算回归；NSH 设置页已补齐自定义色板、命名模板、工具键、工具栏显隐与界面缩放，对丢矩阵无未分类行，截图不再阻塞发布。详见 docs/architecture/nsh-screenshot.md。

## 验收（2026-11-25）

- 本地 Python HTTP 服务 127.0.0.1:8765 提供 manifest 与 zip；宿主启动后自动检查到 0.1.2。
- 点击“下载并重启”：下载、SHA-256、解包、复制到临时 target、启动新 exe 全流程通过；target 出现新 exe 与 nsh/app.json，新进程运行。
- 截图：docs/assets/nsh-release/2026-11-25/update-settings-125.png。
- 自动化：cargo test --lib --offline 包含清单版本/哈希、HTTP 404、哈希不符、目标不可写测试；全部通过。
