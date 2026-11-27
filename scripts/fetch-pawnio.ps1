# Downloads the official signed PawnIO setup into src-tauri/redist/ so the NSIS installer
# can install the driver (see src-tauri/nsis/hooks.nsh). Pinned version + SHA-256.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/fetch-pawnio.ps1

$ErrorActionPreference = 'Stop'

$Version = '2.2.0'
$Sha256 = '1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032'
$Url = "https://github.com/namazso/PawnIO.Setup/releases/download/$Version/PawnIO_setup.exe"

$RedistDir = Join-Path $PSScriptRoot '..\src-tauri\redist'
$Target = Join-Path $RedistDir 'PawnIO_setup.exe'

function Test-Hash($Path) {
    (Get-FileHash -Algorithm SHA256 -Path $Path).Hash -eq $Sha256.ToUpperInvariant()
}

if ((Test-Path $Target) -and (Test-Hash $Target)) {
    Write-Host "PawnIO $Version already present: $Target"
    exit 0
}

New-Item -ItemType Directory -Force -Path $RedistDir | Out-Null
$Tmp = "$Target.download"
Write-Host "Downloading PawnIO $Version from $Url"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $Url -OutFile $Tmp -UseBasicParsing

if (-not (Test-Hash $Tmp)) {
    $actual = (Get-FileHash -Algorithm SHA256 -Path $Tmp).Hash
    Remove-Item $Tmp -Force
    throw "PawnIO_setup.exe hash mismatch. Expected $Sha256, got $actual"
}

Move-Item -Force $Tmp $Target
Write-Host "PawnIO $Version saved to $Target"
