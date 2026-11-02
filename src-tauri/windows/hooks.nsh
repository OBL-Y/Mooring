; Extra steps for the Tauri NSIS installer.
; The installer itself already offers "Just me" (no admin) or "All users" (admin),
; creates the Start menu entry, and offers a desktop shortcut on the finish page.

!macro NSIS_HOOK_PREUNINSTALL
  ; Mooring turns on "run at startup" for each user the first time it runs.
  ; Remove that entry for the uninstalling user so nothing is left behind.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Mooring"
!macroend
