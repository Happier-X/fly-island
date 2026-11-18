# 双屏截图：笔记本内置屏整块黑的排查与修复

日期：2026-11-18。代码基线：`c5d7047`（v3.4.0）。复现与修复后验证都在同一台机器上完成。

## 现象

笔记本外接显示器，按下截图热键（`Ctrl+Alt+A`）后：

- 外接主屏正常：暗化桌面 + 选区 + 工具栏；
- 笔记本内置屏整块纯黑，看不到暗化桌面，只能 `Esc` 退出。

用户报告版本 v3.4.0；`crates/airsnip` 自 v3.3.0 起没有相关改动，说明问题是既有路径在新显示器配置下才暴露。

## 实测环境

| 屏 | 设备 | 物理矩形（PerMonitorV2） | 缩放 | DisplayConfig 高级色彩 | DXGI 桌面色彩空间 |
| --- | --- | --- | --- | --- | --- |
| `\\.\DISPLAY2`（主） | LECOOK2718Q-2（LEC2798） | `0,0 2560×1440` | 125% | 支持，未启用 | `RGB_FULL_G22_NONE_P709`（SDR） |
| `\\.\DISPLAY1`（副） | 笔记本内置屏（LEN9121） | `2560,41 2240×1400` | 150% | **支持且已启用** | **`RGB_FULL_G22_NONE_P709`（SDR）** |

虚拟屏：`0,0 4800×1441`（两屏分辨率不同、DPI 不同、顶部错开 41px）。

## 复现方式

将两块显示器设为扩展桌面，外接屏为主屏，内置屏位于右侧并向下错开 41 个物理像素。用 v3.4.0 的截图快捷键触发覆盖层，对比两块屏上的冻结画面。排查时在 Per-Monitor V2 进程中调用 `capture::capture_virtual_screen()`，按各屏的物理矩形统计全黑像素占比。当前版本可用 `AIRSNIP_DEBUG_CAPTURE=1` 查看每块屏的高级色彩状态与补抓路径。

## 证据

### 1. 整屏抓取：内置屏区域 100% 全 0

用与覆盖层完全相同入口 `capture::capture_virtual_screen()` 抓一次并逐屏统计（`AIRSNIP_DEBUG_CAPTURE=1`）：

```text
[airsnip] 虚拟屏 0,0 4800×1441
[airsnip] 显示器 \\.\DISPLAY1 2560,41 2240×1400：DisplayConfig 报 HDR=true
[airsnip] 显示器 \\.\DISPLAY2 0,0 2560×1440：DisplayConfig 报 HDR=false
[airsnip] HDR 输出在 2560,41 2240×1400，白点 80nits
[airsnip] 补抓跳过 \\.\DISPLAY1 2560,41 2240×1400：已由 HDR 路径填入
[airsnip] 补抓 \\.\DISPLAY2 0,0 2560×1440：BitBlt 完成
显示器 #0 2560,41 2240×1400：全黑 100.0%，平均亮度 0.0
显示器 #1 0,0 2560×1440：全黑 0.0%，平均亮度 236.4
```

导出整屏 BMP 后也能直接看到：左半（外接屏）是真实桌面，右半（内置屏）是纯黑。
说明**黑屏发生在抓屏阶段，不是覆盖层上屏阶段**。

### 2. DDA 与 BitBlt 对同一块屏的结果不同

对两个 output 分别做 `DuplicateOutput1`（FP16 与 BGRA8 各试一次）和自己的 GDI `BitBlt`：

| 输出 | DDA FP16 | DDA BGRA8 | BitBlt |
| --- | --- | --- | --- |
| `\\.\DISPLAY2`（主屏） | 首帧 `LastPresentTime=0`（丢弃重试），第二帧正常 | 正常 | 全黑 0.0%，均值 236 |
| `\\.\DISPLAY1`（内置屏） | `LastPresentTime≠0` 但**整帧全 0** | 同样全 0 | **全黑 0.0%，均值 247** |

GDI 能正常抓到内置屏内容，DDA 抓这块屏只有黑帧（并且随后 `AcquireNextFrame` 立刻返回
`0x887A0026 DXGI_ERROR_ACCESS_LOST`）。

### 3. 「高级色彩已启用」不等于 HDR 合成

`display_probe` 同时读 DisplayConfig 与 DXGI：

```text
\\.\DISPLAY1: 高级色彩 supported=true enabled=true SDR白点=1000   ← 内置屏
\\.\DISPLAY2: 高级色彩 supported=true enabled=false SDR白点=1000  ← 外接屏
\\.\DISPLAY1: DXGI 桌面色彩空间=DXGI_COLOR_SPACE_TYPE(0) bpc=8    ← RGB_FULL_G22_NONE_P709
\\.\DISPLAY2: DXGI 桌面色彩空间=DXGI_COLOR_SPACE_TYPE(0) bpc=8
```

内置屏的 `AdvancedColorEnabled` 为 true，同时 DXGI 报告 P709/SDR（`ColorSpace=0`、`bpc=8`）。自动颜色管理（ACM）可能产生这种组合，本机的 ACM 设置开关未确认。

## 根因

1. `capture/hdr/output.rs` 只看 DisplayConfig 的 `AdvancedColorEnabled` 就决定走 DDA，
   把这块**广色域 SDR 屏**当成了 HDR 输出；
2. `capture/hdr/mod.rs` 把它抓到的全黑帧当成正常内容贴进画布，并把它记进 `covered`；
3. `capture/mod.rs` 的 `fill_uncovered_monitors` 见它「已被 HDR 路径覆盖」就跳过，不补 BitBlt。

三步叠加，画布上这块屏保持全 0，覆盖层照原样上屏 → 整块黑。
坐标运算本身没有问题：`monitor_rects` / `blit_region_into` 的交集与偏移在两屏错开 41px 的真实布局下正确。

## 修复

`crates/airsnip/src/capture/`：

- `hdr/output.rs`：新增 `is_hdr_color_space()`，DDA 的准入条件改为
  **DisplayConfig 已启用高级色彩 且 DXGI 桌面色彩空间是 HDR/scRGB**
  （`RGB_FULL_G10_NONE_P709` scRGB、`RGB_FULL_G2084_NONE_P2020` HDR10）。
  广色域 SDR（`RGB_FULL_G22_NONE_P2020` 等）交给 BitBlt，颜色才是对的。
- `hdr/mod.rs`：整屏抓取时逐 output 容错——
  单块屏抓帧失败、或拿到整帧全 0，只跳过这一块并打印日志，不再让整次截图失败；
  `covered` 只记录**真正贴进画布**的输出，剩下的交给调用方 BitBlt 补抓。
  区域抓取（长截图 / 贴图编辑）遇到失败或全黑帧返回 `None`，回退 BitBlt。
- `capture/mod.rs`：把多显示器枚举、补抓与画布拼装拆到新文件 `capture/monitors.rs`
  （`MONITORINFOEXW` 带设备名），新增 `AIRSNIP_DEBUG_CAPTURE=1` 诊断日志：
  每块显示器的矩形、设备名、DisplayConfig HDR 状态、补抓与否。

## 验证（同一台机器）

- 修复后同一入口：`无 HDR 输出，整屏走 BitBlt`，两块屏 `全黑 0.0%`
  （内置屏均值 244.5、外接屏 234.4），导出 BMP 两半都是真实桌面。
- 真机覆盖层（`pnpm tauri build --no-bundle` 构建，实机触发 `Ctrl+Alt+A`）：
  - 无选区时两块屏都暗化，暗化比 0.524（外接）/ 0.525（内置），与 `DIM_K/256 = 0.527` 一致；
  - 在内置屏上框选后，选区内部亮度比 **1.00**、其余 0.53，工具栏与 `961 × 548` 尺寸提示都在内置屏上；
  - 内置屏全黑像素比例 0.0%。
- **真 HDR 混合场景也复测了**：用 `DisplayConfigSetDeviceInfo`
  （`DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE`）把外接屏的 HDR 临时打开后，
  `DISPLAY2` 变成 `ColorSpace=12`（HDR10）、`BitsPerColor=10`、SDR 白点 3000（240 nits），
  内置屏仍是 SDR，正好是「真 HDR + SDR」混合配置：

  ```text
  [airsnip] HDR 输出在 0,0 2560×1440，白点 240nits
  [airsnip] 补抓 \\.\DISPLAY1 2560,41 2240×1400：BitBlt 完成
  [airsnip] 补抓跳过 \\.\DISPLAY2 0,0 2560×1440：已由 HDR 路径填入
  显示器 #0 2560,41 2240×1400：全黑 0.0%，平均亮度 162.5   ← SDR 屏走 BitBlt
  显示器 #1 0,0 2560×1440：全黑 0.0%，平均亮度 234.9     ← HDR 屏走 DDA + 色调映射
  ```

  覆盖层同样正常：HDR 屏与笔记本屏暗化比都是 0.525，选区 1.000。验证完已把外接屏 HDR 关回原状态。
- 单元测试（`cargo test --manifest-path crates/airsnip/Cargo.toml`，389 项全绿）新增：
  `dda_is_only_for_hdr_color_spaces`、`blank_frame_needs_every_pixel_zero`、
  `neighbours_sharing_an_edge_do_not_overlap`、`negative_layout_overlap_is_symmetric`、
  `covering_rect_of_any_size_counts`、`blit_region_into_handles_negative_canvas_origin`
  （覆盖副屏在右/在左/在上、顶部错开、两种 DPI 的坐标换算）。
- `cargo test --manifest-path src-tauri/Cargo.toml`、`pnpm check`、`pnpm tauri build --no-bundle` 均通过。

## 仍未验证 / 已知限制

- **scRGB（`RGB_FULL_G10_NONE_P709`）真机未测**：本机 HDR 打开后是 HDR10（`ColorSpace=12`），
  scRGB 分支只有单元测试守门。
- **纯 HDR 单屏 / 无 SDR 副屏**：本次在同一台机器上验证的是「HDR + SDR」混合，
  DDA 输出本身正确（HDR 那块屏颜色与亮度正常），但拔掉另一块屏的纯单屏组合没有单独跑。
- 区域抓取（`hdr::capture_region` / `HdrRegionSession`）只取**一个**相交的 HDR 输出；
  请求区域跨两块屏时，另一半在真 HDR 双屏下仍会是黑的（既有行为，本次未改，长截图视口通常在单屏内）。
- 长截图与贴图编辑的实机复测未做：本次只给区域抓取加了「失败 / 全黑帧 → 回退 BitBlt」，
  长截图逐帧拼接在 HDR 屏上原本就正常（ADR 0029），本次没改它的链路。
- 扫描期间显示布局变化（改分辨率 / 拔插屏）没有专门处理：覆盖层用的是抓屏那一刻的虚拟屏快照，
  中途重排会让后续坐标对不上；这属于既有行为。
