!macro NSIS_HOOK_PREINSTALL
  nsExec::ExecToLog 'taskkill /F /IM "sing-box.exe" /T'
  nsExec::ExecToLog 'taskkill /F /IM "xray.exe" /T'
  nsExec::ExecToLog 'taskkill /F /IM "vxray-rust.exe" /T'
!macroend
