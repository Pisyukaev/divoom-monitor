; Installer hooks for the HardwareMonitorCli sidecar.
;
; The sidecar runs as a LocalSystem Windows service so it can use the PawnIO kernel
; driver (CPU temperatures) without the app ever asking for admin rights. Admin rights
; are needed only here, at install time (installMode = perMachine).

!macro NSIS_HOOK_PREINSTALL
  ; On update/reinstall the running service locks its files; stop it first.
  ; `net stop` waits for the service to stop (sc stop does not). Fails harmlessly if absent.
  DetailPrint "Stopping HardwareMonitorCli service..."
  nsExec::ExecToLog 'net stop "HardwareMonitorCli"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; PawnIO is a shared, signed driver (also used by LibreHardwareMonitor and others).
  ; Its setup handles "already installed" and upgrades by itself.
  DetailPrint "Installing PawnIO driver..."
  nsExec::ExecToLog '"$INSTDIR\redist\PawnIO_setup.exe" -install -silent'

  DetailPrint "Registering HardwareMonitorCli as Windows Service..."
  ; binPath is quoted: the install dir contains spaces (unquoted service paths are a security issue).
  nsExec::ExecToLog 'sc create "HardwareMonitorCli" binPath= "\"$INSTDIR\sidecar\HardwareMonitorCli.exe\"" start= auto DisplayName= "Hardware Monitor CLI"'
  ; If the service already existed (update or a dev registration), point it at this install.
  nsExec::ExecToLog 'sc config "HardwareMonitorCli" binPath= "\"$INSTDIR\sidecar\HardwareMonitorCli.exe\"" start= auto'
  nsExec::ExecToLog 'sc description "HardwareMonitorCli" "Provides CPU/GPU metrics for Divoom Monitor"'
  nsExec::ExecToLog 'sc failure "HardwareMonitorCli" reset= 60 actions= restart/5000/restart/10000/restart/30000'
  nsExec::ExecToLog 'sc start "HardwareMonitorCli"'
  DetailPrint "HardwareMonitorCli service registered."
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing HardwareMonitorCli service..."
  nsExec::ExecToLog 'net stop "HardwareMonitorCli"'
  nsExec::ExecToLog 'sc delete "HardwareMonitorCli"'
  ; PawnIO is intentionally left installed: other apps may rely on it.
  DetailPrint "HardwareMonitorCli service removed."
!macroend
