# 生成给 Tauri 更新器用的 NSIS 安装包（老用户在线更新到 NSH 版）。
# 输入是 package.ps1 已构建好的 portable 目录，不重新编译、不改动 NSH 产物。
# 例：pwsh -File nsh/scripts/tauri-bridge.ps1 -PackageDir nsh\.package\dist-0.2.0\portable
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDir,
    [string]$OutputDir,
    [string]$BridgeVersion = '3.6.1',
    [string]$MakensisPath = "$env:LOCALAPPDATA\tauri\NSIS\makensis.exe"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$nshDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$package = [IO.Path]::GetFullPath($PackageDir)
$config = Get-Content -LiteralPath (Join-Path $nshDir 'nsh.json') -Raw | ConvertFrom-Json
$appVersion = [string]$config.version
if (-not $OutputDir) {
    $OutputDir = Split-Path -Parent $package
}
$out = [IO.Path]::GetFullPath($OutputDir)
New-Item -ItemType Directory -Path $out -Force | Out-Null

if (-not (Test-Path -LiteralPath $MakensisPath -PathType Leaf)) {
    throw "找不到 makensis.exe：$MakensisPath"
}
foreach ($required in @(
        'focusd-island-host.exe', 'app.ico', 'LICENSE', 'NOTICE',
        'AIRSNIP-THIRD-PARTY.md', 'UNSIGNED.txt', 'nsh\app.json', 'nsh\assets'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $package $required))) {
        throw "portable 目录缺少 $required；先跑 package.ps1。"
    }
}

$base = "fly-island_${BridgeVersion}_x64-setup"
& $MakensisPath `
    "/DAppVersion=$appVersion" `
    "/DBridgeVersion=$BridgeVersion" `
    "/DPackageDir=$package" `
    "/DOutputDir=$out" `
    "/DOutFileBase=$base" `
    (Join-Path $nshDir 'installer\tauri-update.nsi')
if ($LASTEXITCODE -ne 0) {
    throw "makensis 失败，退出码 $LASTEXITCODE。"
}

$exe = Join-Path $out "$base.exe"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "未生成 $exe"
}
$item = Get-Item -LiteralPath $exe
[pscustomobject]@{
    Path = $item.FullName
    Bytes = $item.Length
    Sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    BridgeVersion = $BridgeVersion
    AppVersion = $appVersion
} | Format-List
