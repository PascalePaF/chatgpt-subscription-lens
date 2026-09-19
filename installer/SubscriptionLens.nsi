Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !define VERSION "1.1.1"
!endif

!define APP_NAME "订阅镜"
!define APP_EXE "SubscriptionLens.exe"
!define APP_ID "SubscriptionLens"
!define PUBLISHER "PascalePaF"
!define PROJECT_URL "https://github.com/PascalePaF/chatgpt-subscription-lens"

Name "${APP_NAME} ${VERSION}"
OutFile "..\release\SubscriptionLens-v${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\SubscriptionLens"
InstallDirRegKey HKCU "Software\${APP_ID}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${APP_NAME} · 本地只读订阅查询"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=2052 "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=2052 "FileDescription" "${APP_NAME} Windows 安装程序"
VIAddVersionKey /LANG=2052 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=2052 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" "Copyright © 2026 ${PUBLISHER}"

!define MUI_ICON "..\native\icons\icon.ico"
!define MUI_UNICON "..\native\icons\icon.ico"
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
    Sleep 750
  LegacyCleanupDone:
  Delete "$SMPROGRAMS\订阅镜.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\订阅镜"
  Delete "$LOCALAPPDATA\订阅镜\chatgpt-subscription-lens.exe"
  Delete "$LOCALAPPDATA\订阅镜\uninstall.exe"
  RMDir /r "$LOCALAPPDATA\订阅镜\subscription-lens-data"
  RMDir "$LOCALAPPDATA\订阅镜"
FunctionEnd

Section "安装 ${APP_NAME}" SecMain
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File /oname=${APP_EXE} "..\native\target\release\chatgpt-subscription-lens.exe"
  File /oname=LICENSE.txt "..\LICENSE"
  File /oname=PRIVACY.md "..\PRIVACY.md"
  File /oname=THIRD_PARTY_NOTICES.md "..\THIRD_PARTY_NOTICES.md"
  File /oname=OFL-Noto-CJK.txt "..\assets\fonts\OFL-Noto-CJK.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\订阅镜"
  CreateShortcut "$SMPROGRAMS\订阅镜\订阅镜.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
  CreateShortcut "$SMPROGRAMS\订阅镜\卸载订阅镜.lnk" "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\${APP_ID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayVersion" "${VERSION}"
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

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\PRIVACY.md"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\OFL-Noto-CJK.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
  DeleteRegKey HKCU "Software\${APP_ID}"
SectionEnd
