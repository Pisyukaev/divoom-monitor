# Takes a webcam photo of the device so the result of a command can be checked.
# Needs ffmpeg in PATH (winget install Gyan.FFmpeg).
#
#   ./scripts/capture-device.ps1                    # one photo -> $env:TEMP\divoom-monitor\camera\device.jpg
#   ./scripts/capture-device.ps1 -Count 4 -Interval 0.5   # several photos, e.g. for animations
#   ./scripts/capture-device.ps1 -ListCameras
#
# Dev builds also save what was sent to each screen in $env:TEMP\divoom-monitor\sent\
# (screen<N>.jpg + log.txt), see draw_commands.rs#dump_sent_image.
param(
    [string]$Camera = $(if ($env:DIVOOM_CAMERA) { $env:DIVOOM_CAMERA } else { 'Logitech StreamCam' }),
    [string]$OutDir = (Join-Path $env:TEMP 'divoom-monitor\camera'),
    [string]$Size = '1920x1080',
    [int]$Count = 1,
    [double]$Interval = 1,
    # Seconds skipped at start so auto exposure and focus settle.
    [double]$Warmup = 1.5,
    [switch]$ListCameras
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error 'ffmpeg not found in PATH. Install it: winget install Gyan.FFmpeg'
}

if ($ListCameras) {
    # ffmpeg prints the device list to stderr and exits with an error; that is expected.
    $ErrorActionPreference = 'Continue'
    ffmpeg -hide_banner -list_devices true -f dshow -i dummy 2>&1 | ForEach-Object { "$_" } |
        Where-Object { $_ -match '\(video\)' }
    exit 0
}

New-Item -ItemType Directory -Force $OutDir | Out-Null

if ($Count -le 1) {
    $out = Join-Path $OutDir 'device.jpg'
    ffmpeg -hide_banner -loglevel error -y -f dshow -video_size $Size -i "video=$Camera" `
        -ss $Warmup -frames:v 1 -q:v 2 $out
} else {
    Get-ChildItem $OutDir -Filter 'device_*.jpg' -ErrorAction SilentlyContinue | Remove-Item -Force
    $out = Join-Path $OutDir 'device_%02d.jpg'
    ffmpeg -hide_banner -loglevel error -y -f dshow -video_size $Size -i "video=$Camera" `
        -ss $Warmup -vf "fps=1/$Interval" -frames:v $Count -q:v 2 $out
}

if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Get-ChildItem $OutDir -Filter 'device*.jpg' | Sort-Object Name | ForEach-Object { $_.FullName }
