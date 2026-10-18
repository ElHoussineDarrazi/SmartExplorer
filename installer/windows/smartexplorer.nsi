; =============================================================
; SmartExplorer — Installateur Windows (NSIS)
;
; Compile avec :
;   makensis /DVERSION=<version> smartexplorer.nsi
; Variables (via /D, optionnelles) :
;   VERSION      Version de l'application (défaut : 0.0.0)
;   SOURCE_DIR   Dossier contenant smart_explorer.exe (défaut : ..\target\release)
;   OUT_DIR      Dossier de sortie de l'installateur (défaut : ..\dist)
;   L'icône est lue dans ${OUT_DIR}\app.ico si présente (générée en amont).
; =============================================================

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SOURCE_DIR
  !define SOURCE_DIR "..\target\release"
!endif
!ifndef OUT_DIR
  !define OUT_DIR "..\dist"
!endif

!define APP_NAME "SmartExplorer"
!define EXE_NAME "SmartExplorer.exe"
!define PUBLISHER "SmartExplorer"
!define URL "https://github.com/ElHoussineDarrazi/SmartExplorer"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

; ---- Paramètres généraux ------------------------------------------
Name "${APP_NAME} ${VERSION}"
OutFile "${OUT_DIR}\SmartExplorer-windows-setup.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"
RequestExecutionLevel admin
Unicode true
SetCompressor /SOLID lzma

; ---- Pages ---------------------------------------------------------
Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

; ---- Installation --------------------------------------------------
Section "Installation" SecInstall
  SetOutPath "$INSTDIR"

  ; Binaire (renommé SmartExplorer.exe pour un lisible propre).
  File /oname=${EXE_NAME} "${SOURCE_DIR}\smart_explorer.exe"

  ; Icône : PNG converti en .ico en amont par ImageMagick (step CI).
  ; Optionnel : si absente, on garde l'icône par défaut de l'EXE.
  IfFileExists "${OUT_DIR}\app.ico" 0 +3
    File "${OUT_DIR}\app.ico"
    ; rien d'autre à faire, l'icône sert d'icône du désinstalleur

  ; Désinstalleur.
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Entrée "Programmes et fonctionnalités".
  WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "URLInfoAbout" "${URL}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXE_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1

  ; Raccourci menu Démarrer.
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Désinstaller.lnk" "$INSTDIR\Uninstall.exe"

  ; Raccourci bureau.
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"
SectionEnd

; ---- Désinstallation ----------------------------------------------
Section "Uninstall"
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\app.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Désinstaller.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$DESKTOP\${APP_NAME}.lnk"

  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
SectionEnd