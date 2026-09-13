; IA Usage — NSIS uninstall extension.
;
; Included via bundle.windows.nsis.installerHooks (NOT a template fork:
; the stock Tauri installer.nsi stays untouched, so Tauri upgrades keep
; working). This macro is expanded inside Section Uninstall, AFTER the
; stock cleanup (exe, uninstall.exe, shortcuts, registry, and — when the
; checkbox is selected — $APPDATA/$LOCALAPPDATA\<bundle-id>).
;
; Safety contract (release blocker):
; - Runs ONLY when the "delete my settings" checkbox is checked
;   ($DeleteAppDataCheckboxState == 1) AND this is not an update
;   ($UpdateMode <> 1). Updates must never wipe user data.
; - Deletes ONLY the exact whitelisted IA Usage paths below.
;   Missing dir = skip, never an error.
; - Deletes ONLY exact Credential Manager targets of the form
;   "<vendor-slug>.com.alberth.iausagebar". keyring v3 on Windows maps
;   Entry::new(service, user) to target "<user>.<service>", and our
;   service/user pairs are frozen (config::CREDENTIAL_SERVICE +
;   VendorId slugs). No wildcards, no enumeration, no fuzzy matching.
; - NEVER touches ~/.claude, ~/.codex, Cursor/VSCode state, gh auth,
;   Google/external credentials, or anything outside IA Usage.

; The CLI is bundled by Tauri at $INSTDIR\resources\bin\iausage.exe. Add
; exactly that per-user directory, never the machine PATH. Both install and
; uninstall compare complete semicolon-delimited segments: `bin` must never
; match `binary`, and an update must never append a duplicate.
!include "LogicLib.nsh"
!include "WinMessages.nsh"

Function AddCliToPath
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCpy $6 $0
  StrCpy $1 "$INSTDIR\resources\bin"
  StrCpy $2 ""
  StrCpy $3 ""
  StrCpy $5 "0"

path_add_next:
  StrCpy $4 $0 1
  StrCmp $4 "" path_add_finish
  StrCmp $4 ";" path_add_boundary
  StrCpy $3 "$3$4"
  StrCpy $0 $0 "" 1
  Goto path_add_next

path_add_boundary:
  StrCmp $3 "" path_add_advance
  StrCmp $3 $1 path_add_target path_add_other
path_add_target:
  StrCmp $5 "1" path_add_advance
  StrCpy $3 $1
  StrCpy $5 "1"
path_add_other:
  StrCmp $2 "" path_add_first path_add_more
path_add_first:
  StrCpy $2 "$3"
  Goto path_add_advance
path_add_more:
  StrCpy $2 "$2;$3"
path_add_advance:
  StrCpy $3 ""
  StrCpy $0 $0 "" 1
  Goto path_add_next

path_add_finish:
  StrCmp $3 "" path_add_missing
  StrCmp $3 $1 path_add_last_target path_add_last_other
path_add_last_target:
  StrCmp $5 "1" path_add_write
  StrCpy $3 $1
  StrCpy $5 "1"
path_add_last_other:
  StrCmp $2 "" path_add_last_first path_add_last_more
path_add_last_first:
  StrCpy $2 "$3"
  Goto path_add_missing
path_add_last_more:
  StrCpy $2 "$2;$3"
path_add_missing:
  StrCmp $5 "1" path_add_write
  StrCmp $2 "" path_add_only path_add_append
path_add_only:
  StrCpy $2 "$1"
  Goto path_add_write
path_add_append:
  StrCpy $2 "$2;$1"
path_add_write:
  StrCmp $2 $6 path_add_done
  WriteRegExpandStr HKCU "Environment" "Path" "$2"
path_add_done:
FunctionEnd

; StrFunc only ships the install-section version of StrRep in the NSIS
; distribution used by Tauri. Keep the uninstall self-contained: split the
; user PATH, retain every segment except our exact CLI directory, then write
; the reconstructed value back.
Function un.RemoveCliFromPath
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCpy $1 "$INSTDIR\resources\bin"
  StrCpy $2 ""
  StrCpy $3 ""

un_path_next:
  StrCpy $4 $0 1
  StrCmp $4 "" un_path_finish
  StrCmp $4 ";" un_path_boundary
  StrCpy $3 "$3$4"
  StrCpy $0 $0 "" 1
  Goto un_path_next

un_path_boundary:
  StrCmp $3 "" un_path_advance
  StrCmp $3 $1 un_path_advance
  StrCmp $2 "" un_path_append_first un_path_append_next
un_path_append_first:
  StrCpy $2 "$3"
  Goto un_path_advance
un_path_append_next:
  StrCpy $2 "$2;$3"
un_path_advance:
  StrCpy $3 ""
  StrCpy $0 $0 "" 1
  Goto un_path_next

un_path_finish:
  StrCmp $3 "" un_path_write
  StrCmp $3 $1 un_path_write
  StrCmp $2 "" un_path_last_first un_path_last_next
un_path_last_first:
  StrCpy $2 "$3"
  Goto un_path_write
un_path_last_next:
  StrCpy $2 "$2;$3"
un_path_write:
  WriteRegExpandStr HKCU "Environment" "Path" "$2"
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  SetShellVarContext current
  IfFileExists "$INSTDIR\resources\bin\iausage.exe" cli_file_present cli_file_missing
cli_file_missing:
  DetailPrint "ERROR: bundled IA Usage CLI is missing"
  Abort "IA Usage CLI no fue incluido en el instalador. La instalación no puede continuar."
cli_file_present:
  Call AddCliToPath
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode = 0
    SetShellVarContext current
    Call un.RemoveCliFromPath
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current

    ; --- Own data dirs: exact whitelist, skip when absent ---
    ; (The com.alberth.iausagebar roaming/local dirs are already removed
    ; by the stock Section Uninstall above; the ia-usagebar dir is where
    ; config.toml, detect.json, snapshots, logs and oauth files live.)
    ${If} ${FileExists} "$APPDATA\ia-usagebar\*.*"
      RmDir /r "$APPDATA\ia-usagebar"
    ${EndIf}
    ${If} ${FileExists} "$LOCALAPPDATA\ia-usagebar\*.*"
      RmDir /r "$LOCALAPPDATA\ia-usagebar"
    ${EndIf}

    ; --- Own Credential Manager entries: exact targets only ---
    ; One silent exact delete per frozen vendor slug. cmdkey exits
    ; non-zero when the target does not exist; output is discarded and
    ; the uninstaller continues (missing entry = already clean).
    nsExec::ExecToStack 'cmdkey /delete:anthropic.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:anthropic_api.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:openai.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:openai_admin.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:copilot.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:zai.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:openrouter.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:deepseek.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:kimi.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:kilo.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:novita.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:moonshot.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:grok.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:supergrok.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:antigravity.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:cursor.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:minimax.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:kiro.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:nous.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:opencode_go.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:commandcode.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:groq.com.alberth.iausagebar'
    Pop $0
    Pop $0
    nsExec::ExecToStack 'cmdkey /delete:windsurf.com.alberth.iausagebar'
    Pop $0
    Pop $0
  ${EndIf}
!macroend
