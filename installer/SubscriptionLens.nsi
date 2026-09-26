Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "1.0.1"
!endif

!define APP_NAME "订阅镜"
!define APP_EXE "SubscriptionLens.exe"
!define APP_ID "SubscriptionLens"
!define PUBLISHER "PascalePaF"
!define PROJECT_URL "https://github.com/PascalePaF/chatgpt-subscription-lens"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "..\release\SubscriptionLens-v${APP_VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\SubscriptionLens"
InstallDirRegKey HKCU "Software\${APP_ID}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${APP_NAME} · 本机处理 · 只读"

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=2052 "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=2052 "FileDescription" "${APP_NAME} Windows 安装程序"
VIAddVersionKey /LANG=2052 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=2052 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" "Copyright © 2026 ${PUBLISHER}"

!define MUI_ICON "..\assets\SubscriptionLens.ico"
!define MUI_UNICON "..\assets\SubscriptionLens.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "运行 ${APP_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  SetShellVarContext current
  IfFileExists "$LOCALAPPDATA\订阅镜\uninstall.exe" 0 LegacyCleanupDone
    ExecWait '"$LOCALAPPDATA\订阅镜\uninstall.exe" /S' $0
    ${If} $0 != 0
      MessageBox MB_ICONSTOP "无法移除旧版订阅镜。请先关闭旧版程序，再重新运行安装程序。"
      Abort
    ${EndIf}
    Sleep 500
  LegacyCleanupDone:
  Delete "$SMPROGRAMS\订阅镜.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\订阅镜"
  Delete "$LOCALAPPDATA\订阅镜\chatgpt-subscription-lens.exe"
  Delete "$LOCALAPPDATA\订阅镜\uninstall.exe"
  RMDir /r "$LOCALAPPDATA\订阅镜\subscription-lens-data"
  RMDir "$LOCALAPPDATA\订阅镜"
  ; V1.0.0 used .NET single-file extraction. V1.0.1 is multi-file and removes
  ; only its own reproducible legacy runtime cache, never user documents.
  RMDir /r "$TEMP\.net\SubscriptionLens"
FunctionEnd

Section "安装 ${APP_NAME}" SecMain
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ; Remove files that existed in the retired Rust build but are not part of the WPF rebuild.
  Delete "$INSTDIR\OFL-Noto-CJK.txt"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  File /r "..\artifacts\publish\*.*"
  File /oname=LICENSE.txt "..\LICENSE"
  File /oname=README.md "..\README.md"
  File /oname=PRIVACY.md "..\PRIVACY.md"
  File /oname=SECURITY.md "..\SECURITY.md"
  File /oname=THIRD_PARTY_NOTICES.md "..\THIRD_PARTY_NOTICES.md"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\订阅镜"
  CreateShortcut "$SMPROGRAMS\订阅镜\订阅镜.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\订阅镜\卸载订阅镜.lnk" "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\${APP_ID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "URLInfoAbout" "${PROJECT_URL}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayIcon" "$INSTDIR\${APP_EXE},0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\订阅镜\订阅镜.lnk"
  Delete "$SMPROGRAMS\订阅镜\卸载订阅镜.lnk"
  RMDir "$SMPROGRAMS\订阅镜"

  ; The application never writes user data here, so the installation tree can
  ; be removed as one exact, per-user directory, including self-contained runtime files.
  RMDir /r "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
  DeleteRegKey HKCU "Software\${APP_ID}"
SectionEnd
