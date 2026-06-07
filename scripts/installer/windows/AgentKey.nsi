Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "AgentKey"
OutFile "${ROOT}\dist\windows\AgentKey-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\AgentKey"
InstallDirRegKey HKCU "Software\AgentKey" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\agentkey-manager\src-tauri\icons\icon.ico"
!define MUI_UNICON "${ROOT}\apps\agentkey-manager\src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"

  nsExec::ExecToLog 'taskkill /IM agentkey.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM agentkey-manager.exe /F'
  Pop $0

  File "${ROOT}\dist\windows\app\agentkey.exe"
  File "${ROOT}\dist\windows\app\agentkey-manager.exe"

  Delete "$DESKTOP\AgentKey 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\AgentKey\AgentKey 绠＄悊宸ュ叿.lnk"

  CreateShortcut "$DESKTOP\AgentKey.lnk" "$INSTDIR\agentkey.exe" "" "$INSTDIR\agentkey.exe"
  CreateShortcut "$DESKTOP\AgentKey Manager.lnk" "$INSTDIR\agentkey-manager.exe" "" "$INSTDIR\agentkey-manager.exe"
  CreateDirectory "$SMPROGRAMS\AgentKey"
  CreateShortcut "$SMPROGRAMS\AgentKey\AgentKey.lnk" "$INSTDIR\agentkey.exe" "" "$INSTDIR\agentkey.exe"
  CreateShortcut "$SMPROGRAMS\AgentKey\AgentKey Manager.lnk" "$INSTDIR\agentkey-manager.exe" "" "$INSTDIR\agentkey-manager.exe"
  CreateShortcut "$SMPROGRAMS\AgentKey\卸载 AgentKey.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\agentkey-manager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\AgentKey" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "DisplayName" "AgentKey"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "Publisher" "GPTokens"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "DisplayIcon" "$INSTDIR\agentkey-manager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM agentkey.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM agentkey-manager.exe /F'
  Pop $0

  Delete "$DESKTOP\AgentKey.lnk"
  Delete "$DESKTOP\AgentKey Manager.lnk"
  Delete "$DESKTOP\AgentKey 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\AgentKey\AgentKey.lnk"
  Delete "$SMPROGRAMS\AgentKey\AgentKey Manager.lnk"
  Delete "$SMPROGRAMS\AgentKey\AgentKey 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\AgentKey\卸载 AgentKey.lnk"
  RMDir "$SMPROGRAMS\AgentKey"

  Delete "$INSTDIR\agentkey.exe"
  Delete "$INSTDIR\agentkey-manager.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\AgentKey"
  DeleteRegKey HKCU "Software\AgentKey"
SectionEnd
