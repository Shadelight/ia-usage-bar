; IA Usage Bar — NSIS custom language strings (English).
;
; Registered via bundle.windows.nsis.customLanguageFiles.
;
; IMPORTANT (learned from smoke test): Tauri INCLUDES THIS FILE *INSTEAD
; OF* its stock languages/English.nsh, not on top of it. A file with only
; one override leaves every other Tauri LangString undefined and NSIS
; renders those labels EMPTY (observed: blank "Create desktop shortcut"
; checkbox on the finish page). So this file carries the COMPLETE stock
; set, copied from tauri-bundler 2.11.2 (crates/tauri-bundler/src/bundle/
; windows/nsis/languages/English.nsh), with exactly ONE line changed:
; deleteAppData. Keep in sync when upgrading Tauri: diff against the new
; stock file and re-apply the deleteAppData line.
;
; Note: custom files are NOT passed through handlebars, so the stock
; {{product_name}} placeholders are written out literally below.
;
; Intentionally English-only for 0.2.0 (stock installer renders English;
; bilingual installer deferred unless NSIS is observed auto-selecting
; Spanish).

LangString addOrReinstall ${LANG_ENGLISH} "Add/Reinstall components"
LangString alreadyInstalled ${LANG_ENGLISH} "Already Installed"
LangString alreadyInstalledLong ${LANG_ENGLISH} "${PRODUCTNAME} ${VERSION} is already installed. Select the operation you want to perform and click Next to continue."
LangString appRunning ${LANG_ENGLISH} "IA Usage Bar is running! Please close it first then try again."
LangString appRunningOkKill ${LANG_ENGLISH} "IA Usage Bar is running!$\nClick OK to kill it"
LangString chooseMaintenanceOption ${LANG_ENGLISH} "Choose the maintenance option to perform."
LangString choowHowToInstall ${LANG_ENGLISH} "Choose how you want to install ${PRODUCTNAME}."
LangString createDesktop ${LANG_ENGLISH} "Create desktop shortcut"
LangString dontUninstall ${LANG_ENGLISH} "Do not uninstall"
LangString dontUninstallDowngrade ${LANG_ENGLISH} "Do not uninstall (Downgrading without uninstall is disabled for this installer)"
LangString failedToKillApp ${LANG_ENGLISH} "Failed to kill IA Usage Bar. Please close it first then try again"
LangString installingWebview2 ${LANG_ENGLISH} "Installing WebView2..."
LangString newerVersionInstalled ${LANG_ENGLISH} "A newer version of ${PRODUCTNAME} is already installed! It is not recommended that you install an older version. If you really want to install this older version, it's better to uninstall the current version first. Select the operation you want to perform and click Next to continue."
LangString older ${LANG_ENGLISH} "older"
LangString olderOrUnknownVersionInstalled ${LANG_ENGLISH} "An $R4 version of ${PRODUCTNAME} is installed on your system. It's recommended that you uninstall the current version before installing. Select the operation you want to perform and click Next to continue."
LangString silentDowngrades ${LANG_ENGLISH} "Downgrades are disabled for this installer, can't proceed with the silent installer, please use the graphical interface installer instead.$\n"
LangString unableToUninstall ${LANG_ENGLISH} "Unable to uninstall!"
LangString uninstallApp ${LANG_ENGLISH} "Uninstall ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_ENGLISH} "Uninstall before installing"
LangString unknown ${LANG_ENGLISH} "unknown"
LangString webview2AbortError ${LANG_ENGLISH} "Failed to install WebView2! The app can't run without it. Try restarting the installer."
LangString webview2DownloadError ${LANG_ENGLISH} "Error: Downloading WebView2 Failed - $0"
LangString webview2DownloadSuccess ${LANG_ENGLISH} "WebView2 bootstrapper downloaded successfully"
LangString webview2Downloading ${LANG_ENGLISH} "Downloading WebView2 bootstrapper..."
LangString webview2InstallError ${LANG_ENGLISH} "Error: Installing WebView2 failed with exit code $1"
LangString webview2InstallSuccess ${LANG_ENGLISH} "WebView2 installed successfully"
LangString deleteAppData ${LANG_ENGLISH} "Also delete my settings and local application data"
