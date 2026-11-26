# Registers the dev build of the HardwareMonitorCli sidecar as a Windows service, the same
# way the installer does (src-tauri/nsis/hooks.nsh), so `pnpm start` gets CPU temperatures.
#
#   pnpm sidecar:install     build sidecar, install PawnIO, (re)register and start the service
#   pnpm sidecar:uninstall   stop and remove the service (PawnIO stays installed)
#
# Asks for admin rights (UAC) once per run. Rebuilding the sidecar while the service runs
# fails because the exe is locked: run sidecar:install again, it stops the service first.

param(
    [ValidateSet('install', 'uninstall', 'status')]
    [string]$Action = 'status'
)

$ErrorActionPreference = 'Stop'
$ServiceName = 'HardwareMonitorCli'
$Root = Resolve-Path (Join-Path $PSScriptRoot '..')
$SidecarExe = Join-Path $Root 'src-tauri\sidecar\HardwareMonitorCli.exe'
$PawnIoSetup = Join-Path $Root 'src-tauri\redist\PawnIO_setup.exe'

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    (New-Object Security.Principal.WindowsPrincipal $id).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-Svc { Get-Service -Name $ServiceName -ErrorAction SilentlyContinue }

if ($Action -eq 'status') {
    $svc = Get-Svc
    if ($svc) {
        $path = (Get-CimInstance Win32_Service -Filter "Name='$ServiceName'").PathName
        Write-Host "Service: $($svc.Status) ($path)"
    } else {
        Write-Host 'Service: not installed'
    }
    $pawn = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO' -ErrorAction SilentlyContinue
    Write-Host ("PawnIO: " + ($(if ($pawn) { "installed $($pawn.DisplayVersion)" } else { 'not installed' })))
    exit 0
}

# Build/download as the current user, then elevate only for the system changes.
if (-not (Test-Admin)) {
    if ($Action -eq 'install') {
        & (Join-Path $PSScriptRoot 'fetch-pawnio.ps1')
        if (Get-Svc) {
            Write-Host 'Service is running; it will be stopped (elevated) before rebuilding.'
        }
    }
    $argList = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Action $Action"
    # Not `-Wait`: in Windows PowerShell it also waits for descendants (e.g. dotnet build servers).
    $p = Start-Process powershell -Verb RunAs -ArgumentList $argList -PassThru
    $p.WaitForExit()
    exit $p.ExitCode
}

if ($Action -eq 'uninstall') {
    if (Get-Svc) {
        & net stop $ServiceName | Out-Null
        & sc.exe delete $ServiceName | Out-Null
        Write-Host "Service $ServiceName removed."
    } else {
        Write-Host "Service $ServiceName is not installed."
    }
    exit 0
}

# --- install (elevated) ---
if (Get-Svc) { & net stop $ServiceName 2>$null | Out-Null }

Write-Host 'Building sidecar...'
& dotnet publish (Join-Path $Root 'hardware-monitor-cli\HardwareMonitorCli.csproj') -c Release -r win-x64 --self-contained --disable-build-servers -o (Join-Path $Root 'src-tauri\sidecar') | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'dotnet publish failed' }

if (-not (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO' -ErrorAction SilentlyContinue)) {
    Write-Host 'Installing PawnIO driver...'
    $p = Start-Process -FilePath $PawnIoSetup -ArgumentList '-install', '-silent' -PassThru
    $p.WaitForExit()
    if ($p.ExitCode -ne 0) { throw "PawnIO setup exited with $($p.ExitCode)" }
}

$binPath = "`"$SidecarExe`""
if (Get-Svc) {
    & sc.exe config $ServiceName binPath= $binPath start= auto | Out-Host
} else {
    & sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= 'Hardware Monitor CLI' | Out-Host
}
& sc.exe description $ServiceName 'Provides CPU/GPU metrics for Divoom Monitor (dev build)' | Out-Null
& sc.exe failure $ServiceName reset= 60 actions= restart/5000/restart/10000/restart/30000 | Out-Null
& sc.exe start $ServiceName | Out-Host

Write-Host "`nDone. Service $ServiceName runs $SidecarExe"
Start-Sleep -Seconds 3
