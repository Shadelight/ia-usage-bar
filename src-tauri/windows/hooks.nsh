; IA Usage Bar — NSIS uninstall extension.
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
