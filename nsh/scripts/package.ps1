# 使用完整 Win32 宿主打包；不调用会选用默认 runner 的 nsh package。
#requires -Version 7.0
# 完整构建：pwsh -File nsh/scripts/package.ps1
# 复用本脚本产物：pwsh -File nsh/scripts/package.ps1 -SkipBuild
# 仅便携包：加 -PortableOnly；不会生成或运行安装器。
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$PortableOnly,
    [string]$IsccPath = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$nshDir = Join-Path $root 'nsh'
$work = Join-Path $nshDir '.package'
$target = Join-Path $work 'target'
$resources = Join-Path $work 'resources'
$exe = Join-Path $target 'x86_64-pc-windows-msvc\release\focusd-island-host.exe'
$config = Get-Content -LiteralPath (Join-Path $nshDir 'nsh.json') -Raw | ConvertFrom-Json
$version = [string]$config.version
$toolchainText = Get-Content -LiteralPath (Join-Path $root 'rust-toolchain.toml') -Raw
if ($toolchainText -notmatch 'channel\s*=\s*"([^"]+)"') {
    throw '无法读取 rust-toolchain.toml 的 channel。'
}
$toolchain = $Matches[1]

# 版本是文件名和 ISPP 参数的输入边界；本轮只发布三段数字的稳定版。
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw 'nsh.json 的 version 必须是三段数字，如 0.1.0。'
}
foreach ($manifest in @('Cargo.toml', 'host\Cargo.toml')) {
    $line = Select-String -LiteralPath (Join-Path $nshDir $manifest) `
        -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $line -or $line.Matches[0].Groups[1].Value -ne $version) {
        throw "nsh.json 与 nsh/$manifest 版本不一致。"
    }
}
$name = "FocuSD-Island-NSH-$version-win-x64"
$dist = Join-Path $nshDir 'dist'
$release = Join-Path $dist $name
$portable = Join-Path $release 'portable'
$installer = Join-Path $nshDir 'installer\focusd-island.iss'

if (Test-Path -LiteralPath $release) {
    throw "输出目录已存在，请先人工保留或移走，不会覆盖：$release"
}
if (-not $PortableOnly -and -not (Test-Path -LiteralPath $IsccPath -PathType Leaf)) {
    throw '找不到 ISCC.exe。请通过 -IsccPath 指向已安装的 Inno Setup 6；脚本不会安装它。'
}

function Invoke-Tool([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command 失败，退出码 $LASTEXITCODE。"
    }
}

# 包括 ISCC 的临时文件也放副本内；结束后恢复调用方的进程环境。
$envNames = @('CARGO_TARGET_DIR', 'CARGO_NET_OFFLINE', 'TEMP', 'TMP')
$oldEnv = @{}
foreach ($key in $envNames) {
    $oldEnv[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
$tempDir = Join-Path $work 'temp'
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
Push-Location $root
try {
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_NET_OFFLINE = 'true'
    $env:TEMP = $tempDir
    $env:TMP = $tempDir
    $env:Path = (Join-Path $nshDir '.package\nsh-cli\bin') + [IO.Path]::PathSeparator + $env:Path
    if (-not $SkipBuild) {
        # 锁文件 + 离线构建；toolchain、MSVC、Windows SDK 和缓存依赖需提前备好。
        # rustup run 不带 --install：工具链缺失时报错，不触发自动安装。
        Invoke-Tool 'rustup' @(
            'run', $toolchain, 'cargo', 'build', '--manifest-path', (Join-Path $nshDir 'host\Cargo.toml'),
            '--bin', 'focusd-island-host', '--release', '--locked', '--offline',
            '--target', 'x86_64-pc-windows-msvc',
            '--target-dir', $target
        )
        # 只让 NSH 生成原生 app.json / assets，不使用其打包 runner。
        Invoke-Tool 'nsh' @('build', $nshDir, '-o', $resources)
    }

foreach ($inputFile in @($exe, (Join-Path $resources 'app.json'))) {
    if (-not (Test-Path -LiteralPath $inputFile -PathType Leaf)) {
        throw "缺少输入：$inputFile。必须先构建完整宿主和 NSH 资源；不会回退到默认 runner。"
    }
}
$assets = Join-Path $resources 'assets'
if (-not (Test-Path -LiteralPath $assets -PathType Container)) {
    throw "缺少 NSH 生成的资源目录：$assets"
}

# 只创建新版本输出；失败留下的目录由主人检查后处理，不自动删文件。
$packageNsh = Join-Path $portable 'nsh'
New-Item -ItemType Directory -Path $packageNsh -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $portable 'focusd-island-host.exe')
Copy-Item -LiteralPath (Join-Path $resources 'app.json') -Destination $packageNsh
Copy-Item -LiteralPath $assets -Destination $packageNsh -Recurse
Copy-Item -LiteralPath (Join-Path $root 'LICENSE'), (Join-Path $root 'NOTICE') -Destination $portable
Copy-Item -LiteralPath (Join-Path $root 'crates\airsnip\THIRD-PARTY.md') `
    -Destination (Join-Path $portable 'AIRSNIP-THIRD-PARTY.md')

$unsigned = @'
FocuSD Island (NSH) — 未签名发布 / Unsigned release

运行 focusd-island-host.exe；不要替换为 NSH 默认 runner。
Run focusd-island-host.exe, not the NSH default runner.

本程序与安装器未使用 Authenticode 签名。Windows SmartScreen 可能提示风险；
仅在确认发布来源后自行决定是否运行。SHA-256 只用于核对内容，不证明发布者身份。
The app and installer have no Authenticode signature. Windows may warn.
SHA-256 checks content integrity, not publisher identity.

数据目录 / Data: %APPDATA%\com.focusd.island.nsh
卸载保留应用数据，不改旧 Tauri 数据，不配置邮箱。
Uninstall keeps app data and does not change Tauri data or configure email.

许可证见 LICENSE、NOTICE 与 AIRSNIP-THIRD-PARTY.md。
发布者须按 GPL-3.0 提供与此二进制对应的完整源码及构建说明。
See LICENSE, NOTICE and AIRSNIP-THIRD-PARTY.md. Distributors must provide
corresponding source and build instructions under GPL-3.0.
'@
[IO.File]::WriteAllText((Join-Path $portable 'UNSIGNED.txt'), $unsigned, [Text.UTF8Encoding]::new($false))

$zip = Join-Path $release "$name.zip"
Compress-Archive -Path (Join-Path $portable '*') -DestinationPath $zip -CompressionLevel Optimal
$artifacts = @($zip)
if (-not $PortableOnly) {
    Invoke-Tool $IsccPath @(
        "/DAppVersion=$version", "/DPackageDir=$portable", "/DOutputDir=$release", $installer
    )
    $setup = Join-Path $release "$name-setup.exe"
    if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) {
        throw "ISCC 未生成预期文件：$setup"
    }
    $artifacts += $setup
}

# 记录实际打包的宿主，便于 -SkipBuild 时追溯；不声称二进制与源码已验证一致。
$record = [ordered]@{
    version = $version
    host = $exe
    hostSha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    appJsonSha256 = (Get-FileHash -LiteralPath (Join-Path $resources 'app.json') -Algorithm SHA256).Hash.ToLowerInvariant()
    skippedBuild = [bool]$SkipBuild
    signed = $false
}
$record | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $release 'package-inputs.json') -Encoding UTF8
$hashes = foreach ($artifact in $artifacts) {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant(),
        [IO.Path]::GetFileName($artifact)
}
$hashes | Set-Content -LiteralPath (Join-Path $release 'SHA256SUMS.txt') -Encoding ASCII
Write-Host "输出目录：$release"
Write-Host '未签名；未启动程序或安装器。安装、运行和卸载需主人另行验收。'
}
finally {
    Pop-Location
    foreach ($key in $envNames) {
        [Environment]::SetEnvironmentVariable($key, $oldEnv[$key], 'Process')
    }
}
