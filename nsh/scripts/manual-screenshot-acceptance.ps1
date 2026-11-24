# Manual acceptance for the NSH screenshot S1 clipboard path.
#
# Run once, in an unlocked desktop, after the Tauri app has exited:
#   powershell -NoProfile -ExecutionPolicy Bypass -File nsh/scripts/manual-screenshot-acceptance.ps1
#
# The script never kills the Tauri app. It starts the NSH host, waits for one
# real Ctrl+Shift+A capture, then checks CF_DIB and the registered PNG format,
# writes the clipboard PNG to docs/assets/nsh-screenshot/manual-clipboard.png,
# and asks you to paste it into Notepad.

param([switch]$KeepHost)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $repo 'nsh\host\target\debug\focusd-island-host.exe'
$cfgDir = Join-Path $env:APPDATA 'com.focusd.island.nsh\screenshot'
$evidence = Join-Path $repo 'docs\assets\nsh-screenshot\manual-clipboard.png'
$defaultSave = Join-Path $env:USERPROFILE 'Pictures\FocuSD Island'

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class AcceptWin {
  public delegate bool P(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(P cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int m);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();
  [DllImport("user32.dll")] public static extern bool OpenClipboard(IntPtr h);
  [DllImport("user32.dll")] public static extern bool CloseClipboard();
  [DllImport("user32.dll")] public static extern uint RegisterClipboardFormat(string name);
  [DllImport("user32.dll")] public static extern IntPtr GetClipboardData(uint f);
  [DllImport("kernel32.dll")] public static extern IntPtr GlobalSize(IntPtr h);
  [DllImport("kernel32.dll")] public static extern IntPtr GlobalLock(IntPtr h);
  [DllImport("kernel32.dll")] public static extern bool GlobalUnlock(IntPtr h);
}
"@

function Find-Window([string]$class, [bool]$visibleOnly = $false) {
  $script:found = [IntPtr]::Zero
  $cb = [AcceptWin+P]{
    param($h, $l)
    $c = New-Object Text.StringBuilder 256
    [AcceptWin]::GetClassName($h, $c, 256) | Out-Null
    if ($c.ToString() -eq $class -and (!$visibleOnly -or [AcceptWin]::IsWindowVisible($h))) {
      $script:found = $h
    }
    return $true
  }
  [AcceptWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  return $script:found
}

function Wait-Overlay([bool]$wantOpen, [int]$seconds) {
  $deadline = [DateTime]::UtcNow.AddSeconds($seconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    $hwnd = Find-Window 'airsnip.overlay' $true
    if ($wantOpen -and $hwnd -ne [IntPtr]::Zero) { return $true }
    if (!$wantOpen -and $hwnd -eq [IntPtr]::Zero) { return $true }
    Start-Sleep -Milliseconds 150
  }
  return $false
}

function Read-Format([uint32]$format) {
  $handle = [AcceptWin]::GetClipboardData($format)
  if ($handle -eq [IntPtr]::Zero) { return $null }
  $size = [AcceptWin]::GlobalSize($handle).ToInt32()
  if ($size -le 0) { return $null }
  $ptr = [AcceptWin]::GlobalLock($handle)
  if ($ptr -eq [IntPtr]::Zero) { return $null }
  $bytes = New-Object byte[] $size
  [Runtime.InteropServices.Marshal]::Copy($ptr, $bytes, 0, $size)
  [AcceptWin]::GlobalUnlock($handle) | Out-Null
  return ,$bytes
}

function Read-Clipboard() {
  $dib = $null
  $png = $null
  $pngFormat = [AcceptWin]::RegisterClipboardFormat('PNG')
  for ($i = 0; $i -lt 20; $i++) {
    if ([AcceptWin]::OpenClipboard([IntPtr]::Zero)) {
      try {
        $dib = Read-Format 8
        $png = Read-Format $pngFormat
      } finally {
        [AcceptWin]::CloseClipboard() | Out-Null
      }
      break
    }
    Start-Sleep -Milliseconds 100
  }
  return @{ Dib = $dib; Png = $png }
}

function BE32([byte[]]$bytes, [int]$offset) {
  return ([uint32]$bytes[$offset] -shl 24) -bor ([uint32]$bytes[$offset + 1] -shl 16) -bor ([uint32]$bytes[$offset + 2] -shl 8) -bor [uint32]$bytes[$offset + 3]
}

# Preconditions: old app must be gone; do not kill it from here.
$old = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -eq 'fly-island' }
if ($old) {
  Write-Host 'FAIL: Tauri 版 fly-island 仍在运行。请先退出旧版，再重新运行本脚本。' -ForegroundColor Yellow
  Write-Host '本脚本不会自动结束旧版进程。'
  exit 2
}
if (!(Test-Path $exe)) {
  Write-Host "FAIL: 找不到 $exe，请先 cargo build --manifest-path nsh/host/Cargo.toml" -ForegroundColor Yellow
  exit 2
}

# One clean NSH instance and the default Ctrl+Shift+A setting.
Get-Process focusd-island-host -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
[IO.Directory]::CreateDirectory($cfgDir) | Out-Null
[IO.Directory]::CreateDirectory($defaultSave) | Out-Null
$settings = '{"enabled":true,"shortcut":"Ctrl+Shift+A","saveFormat":"png","saveDir":"","autoSave":false}'
[IO.File]::WriteAllText((Join-Path $cfgDir 'settings.json'), $settings, (New-Object Text.UTF8Encoding($false)))

$env:RUST_LOG = 'info'
$hostProc = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 4
if (!(Find-Window 'FocusdIslandHost')) {
  Write-Host 'FAIL: NSH 宿主窗口没有出现。' -ForegroundColor Yellow
  exit 2
}

$before = [AcceptWin]::GetClipboardSequenceNumber()
Write-Host ''
Write-Host '请在 120 秒内完成：' -ForegroundColor Cyan
Write-Host '  1. 按 Ctrl+Shift+A'
Write-Host '  2. 框选一块有文字和颜色的区域'
Write-Host '  3. 按 Enter 复制（不要按 Esc）'
Write-Host ''

if (!(Wait-Overlay $true 120)) {
  Write-Host 'FAIL: 没等到截图覆盖层。检查 Ctrl+Shift+A 是否被其它程序占用。' -ForegroundColor Yellow
  if (!$KeepHost) { Stop-Process -Id $hostProc.Id -Force -ErrorAction SilentlyContinue }
  exit 3
}
if (!(Wait-Overlay $false 120)) {
  Write-Host 'FAIL: 覆盖层没有关闭，可能取消了截图。' -ForegroundColor Yellow
  if (!$KeepHost) { Stop-Process -Id $hostProc.Id -Force -ErrorAction SilentlyContinue }
  exit 3
}
Start-Sleep -Milliseconds 800

$after = [AcceptWin]::GetClipboardSequenceNumber()
if ($after -eq $before) {
  Write-Host 'FAIL: 剪贴板序号没有变化，复制没有写入剪贴板。' -ForegroundColor Yellow
  if (!$KeepHost) { Stop-Process -Id $hostProc.Id -Force -ErrorAction SilentlyContinue }
  exit 4
}

$clip = Read-Clipboard
if (!$clip.Dib) {
  Write-Host 'FAIL: 剪贴板没有 CF_DIB。' -ForegroundColor Yellow
  exit 4
}
if (!$clip.Png) {
  Write-Host 'FAIL: 剪贴板没有注册格式 PNG。' -ForegroundColor Yellow
  exit 4
}

$dibWidth = [BitConverter]::ToInt32($clip.Dib, 4)
$dibHeight = [Math]::Abs([BitConverter]::ToInt32($clip.Dib, 8))
$pngWidth = BE32 $clip.Png 16
$pngHeight = BE32 $clip.Png 20
if ($dibWidth -ne $pngWidth -or $dibHeight -ne $pngHeight) {
  Write-Host ('FAIL: CF_DIB ' + $dibWidth + 'x' + $dibHeight + ' 与 PNG ' + $pngWidth + 'x' + $pngHeight + ' 不一致。') -ForegroundColor Yellow
  exit 4
}

[IO.File]::WriteAllBytes($evidence, $clip.Png)
Write-Host ('PASS: CF_DIB 与 PNG 都是 ' + $dibWidth + 'x' + $dibHeight + '，PNG 已写到 ' + $evidence) -ForegroundColor Green

$notepad = Start-Process notepad -PassThru
Write-Host ''
Read-Host '请在新打开的记事本里按 Ctrl+V 粘贴，确认图像正常后按 Enter'
$notepad.Refresh()
if (!$notepad.HasExited) {
  Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
}

Write-Host 'PASS: 截图、CF_DIB、PNG、记事本粘贴人工步骤完成。' -ForegroundColor Green
if (!$KeepHost) { Stop-Process -Id $hostProc.Id -Force -ErrorAction SilentlyContinue }
