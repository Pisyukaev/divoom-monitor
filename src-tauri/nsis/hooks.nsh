!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Registering HardwareMonitorCli as Windows Service..."
  nsExec::ExecToLog 'sc create "HardwareMonitorCli" binPath= "$INSTDIR\sidecar\HardwareMonitorCli.exe" start= auto DisplayName= "Hardware Monitor CLI"'
  nsExec::ExecToLog 'sc description "HardwareMonitorCli" "Provides CPU/GPU metrics for Divoom Monitor"'
  nsExec::ExecToLog 'sc failure "HardwareMonitorCli" reset= 60 actions= restart/5000/restart/10000/restart/30000'
  nsExec::ExecToLog 'sc start "HardwareMonitorCli"'
  DetailPrint "HardwareMonitorCli service registered."

  ; Live screen values: the Times Gate fetches them from the app over the LAN
  ; (src-tauri/src/live_screen.rs), so inbound connections must be allowed.
  DetailPrint "Allowing Divoom Monitor through Windows Firewall..."
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Divoom Monitor"'
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="Divoom Monitor" dir=in action=allow program="$INSTDIR\divoom-monitor.exe" protocol=TCP enable=yes profile=private,public'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Divoom Monitor"'
  DetailPrint "Removing HardwareMonitorCli service..."
  nsExec::ExecToLog 'sc stop "HardwareMonitorCli"'
  nsExec::ExecToLog 'sc delete "HardwareMonitorCli"'
  DetailPrint "HardwareMonitorCli service removed."
!macroend