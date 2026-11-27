; 由 nsh/scripts/package.ps1 调用，所有输入均为副本内的显式暂存目录。
; 不调用 SignTool，不打包 NSH 默认 runner，不自动启动应用。
#ifndef AppVersion
  #error 缺少 /DAppVersion
#endif
#ifndef PackageDir
  #error 缺少 /DPackageDir
#endif
#ifndef OutputDir
  #error 缺少 /DOutputDir
#endif
#define AppName "FocuSD Island (NSH)"
#define AppExe "focusd-island-host.exe"

[Setup]
; 与 Tauri 的 com.focusd.island 安装标识分离，后续 NSH 版本保持不变。
AppId=com.focusd.island.nsh
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
VersionInfoVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\FocuSD Island NSH
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
DisableDirPage=no
DisableProgramGroupPage=no
AllowNoIcons=yes
UsePreviousAppDir=yes
UsePreviousGroup=yes
UsePreviousTasks=yes
LicenseFile={#PackageDir}\LICENSE
InfoBeforeFile={#PackageDir}\UNSIGNED.txt
OutputDir={#OutputDir}
OutputBaseFilename=FocuSD-Island-NSH-{#AppVersion}-win-x64-setup
SetupIconFile=..\..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Uninstallable=yes
CreateUninstallRegKey=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; 不自动关闭或重启宿主，安装前由主人从托盘退出。
CloseApplications=no
RestartApplications=no

[Languages]
; 使用 Inno 自带语言，不下载语言包。应用本身的中英文不受影响。
Name: "english"; MessagesFile: "compiler:Default.isl"

[Messages]
InfoBeforeLabel=该应用和安装器未进行数字签名。仅在确认来源可信后继续安装。

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut / 创建桌面快捷方式"; Flags: unchecked
Name: "autostart"; Description: "Start with Windows / 开机自动启动"; Flags: checkedonce

[Files]
; 根目录仅一个应用 exe，保证现有 zip 更新器不会选中默认 runner 或安装器。
Source: "{#PackageDir}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\app.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\nsh\app.json"; DestDir: "{app}\nsh"; Flags: ignoreversion
Source: "{#PackageDir}\nsh\assets\*"; DestDir: "{app}\nsh\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PackageDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\AIRSNIP-THIRD-PARTY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\UNSIGNED.txt"; DestDir: "{app}"; Flags: ignoreversion

; 开机自启使用当前用户 Run 键，与宿主托盘菜单的“开机自启”开关共用同一项。
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "FocuSD Island (NSH)"; ValueData: """{app}\{#AppExe}"""; Tasks: autostart; Flags: uninsdeletevalue

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Tasks: desktopicon

; 故意不设置 [Run]：安装结束不自动启动应用。
; 故意不设置 [UninstallDelete]：不递归清理安装目录或用户数据。
; 不注册开机自启，不修改权限/账号/邮箱，也不触碰 Tauri 安装或数据。
; 目录选择页允许自选位置；请选当前用户可写的目录，否则现有在线更新会失败。
; 安装器的卸载日志只跟踪自身安装文件，便携更新增加的文件可能需要人工处理。
