!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Registering HardwareMonitorCli as Windows Service..."
  nsExec::ExecToLog 'sc create "HardwareMonitorCli" binPath= "$INSTDIR\sidecar\HardwareMonitorCli.exe" start= auto DisplayName= "Hardware Monitor CLI"'
  nsExec::ExecToLog 'sc description "HardwareMonitorCli" "Provides CPU/GPU metrics for Divoom Monitor"'
  nsExec::ExecToLog 'sc failure "HardwareMonitorCli" reset= 60 actions= restart/5000/restart/10000/restart/30000'
  nsExec::ExecToLog 'sc start "HardwareMonitorCli"'
  DetailPrint "HardwareMonitorCli service registered."
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing HardwareMonitorCli service..."
  nsExec::ExecToLog 'sc stop "HardwareMonitorCli"'
  nsExec::ExecToLog 'sc delete "HardwareMonitorCli"'
  DetailPrint "HardwareMonitorCli service removed."
!macroend