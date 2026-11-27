; Tauri 更新桥：让仍在 Tauri 3.6.0 上的老用户通过内置更新器换成 NSH 版。
; 由 nsh/scripts/tauri-bridge.ps1 调用；输入为 package.ps1 已生成的 portable 目录。
; 更新器会以 ShellExecute 传入 "/passive /UPDATE"（也可能 /S /UPDATE），两种都按静默处理。
Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef PackageDir
  !error 缺少 /DPackageDir
!endif
!ifndef OutputDir
  !error 缺少 /DOutputDir
!endif
!ifndef AppVersion
  !error 缺少 /DAppVersion
!endif
!ifndef BridgeVersion
  !define BridgeVersion "3.6.1"
!endif
!ifndef OutFileBase
  !define OutFileBase "FocuSD-Island-NSH-bridge-${AppVersion}-x64-setup"
!endif

!define AppName "FocuSD Island"
!define AppExe "focusd-island-host.exe"
!define UninstallKey "Software\Microsoft\Windows\CurrentVersion\Uninstall\com.focusd.island.nsh_is1"
!define RunValue "FocuSD Island (NSH)"
!define OldInstallDir "$LOCALAPPDATA\fly-island"

Name "${AppName}"
OutFile "${OutputDir}\${OutFileBase}.exe"
InstallDir "$LOCALAPPDATA\Programs\FocuSD Island NSH"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${BridgeVersion}.0"
VIAddVersionKey "ProductName" "${AppName}"
VIAddVersionKey "FileDescription" "FocuSD Island installer"
VIAddVersionKey "FileVersion" "${BridgeVersion}"
VIAddVersionKey "ProductVersion" "${BridgeVersion}"
VIAddVersionKey "LegalCopyright" "GPL-3.0"

!define MUI_WELCOMEPAGE_TITLE "FocuSD Island"
!define MUI_WELCOMEPAGE_TEXT "将安装新的 FocuSD Island（NSH 版）并替换旧的 Tauri 版。$\r$\n$\r$\n旧版的安装文件和开机自启项会被移除，应用数据保留。$\r$\n$\r$\n应用与安装器未做数字签名。"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

; 更新器只给参数，没人点按钮：/passive 或 /S 一律静默安装。
Function .onInit
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/passive" $R1
  ${IfNot} ${Errors}
    SetSilent silent
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/S" $R1
  ${IfNot} ${Errors}
    SetSilent silent
  ${EndIf}
FunctionEnd

Section "Install"
  SetShellVarContext current
  ; 先关掉两个版本可能还在跑的进程，否则 exe 被占用无法替换。
  nsExec::Exec 'taskkill /F /IM focusd-island.exe'
  Pop $0
  nsExec::Exec 'taskkill /F /IM ${AppExe}'
  Pop $0

  ; 迁移：移除旧 Tauri 版的删除残留、开机自启和卸载项。
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "FocuSD Island"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\fly-island"
  RMDir /r "${OldInstallDir}"
  Delete "$SMPROGRAMS\fly-island.lnk"
  Delete "$SMPROGRAMS\${AppName}.lnk"
  RMDir /r "$SMPROGRAMS\fly-island"

  SetOutPath "$INSTDIR"
  File "${PackageDir}\${AppExe}"
  File "${PackageDir}\app.ico"
  File "${PackageDir}\LICENSE"
  File "${PackageDir}\NOTICE"
  File "${PackageDir}\AIRSNIP-THIRD-PARTY.md"
  File "${PackageDir}\UNSIGNED.txt"
  SetOutPath "$INSTDIR\nsh"
  File "${PackageDir}\nsh\app.json"
  SetOutPath "$INSTDIR\nsh\assets"
  File /r "${PackageDir}\nsh\assets\*.*"
  SetOutPath "$INSTDIR"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\${AppName}.lnk" "$INSTDIR\${AppExe}" "" "$INSTDIR\${AppExe}" 0
  CreateShortcut "$DESKTOP\${AppName}.lnk" "$INSTDIR\${AppExe}" "" "$INSTDIR\${AppExe}" 0

  ; 与托盘菜单的“开机自启”共用同一个 Run 值。
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${RunValue}" '"$INSTDIR\${AppExe}"'

  WriteRegStr HKCU "${UninstallKey}" "DisplayName" "${AppName} (NSH)"
  WriteRegStr HKCU "${UninstallKey}" "DisplayVersion" "${AppVersion}"
  WriteRegStr HKCU "${UninstallKey}" "DisplayIcon" "$INSTDIR\${AppExe}"
  WriteRegStr HKCU "${UninstallKey}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UninstallKey}" "Publisher" "Happier-X"
  WriteRegStr HKCU "${UninstallKey}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UninstallKey}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UninstallKey}" "NoModify" 1
  WriteRegDWORD HKCU "${UninstallKey}" "NoRepair" 1

  ; 更新器退出旧进程后由这里把新版拉起来，主人看到的就是更新完成。
  Exec '"$INSTDIR\${AppExe}"'
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  nsExec::Exec 'taskkill /F /IM ${AppExe}'
  Pop $0
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${RunValue}"
  DeleteRegKey HKCU "${UninstallKey}"
  Delete "$SMPROGRAMS\${AppName}.lnk"
  Delete "$DESKTOP\${AppName}.lnk"
  RMDir /r "$INSTDIR"
SectionEnd
