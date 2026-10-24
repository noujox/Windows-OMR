param(
    [Parameter(Mandatory=$true)][string]$ExtensionId,
    [ValidateSet('Chrome','Edge','Both')][string]$Browser = 'Both'
)
$ErrorActionPreference = 'Stop'
if ($ExtensionId -cnotmatch '^[a-p]{32}$') {
    throw 'ExtensionId must be the 32-character ID shown in chrome://extensions or edge://extensions.'
}
$hostName = 'com.computux.meeting_recorder_satellite'
$folder = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $folder 'meeting-recorder-satellite-host.exe'
if (!(Test-Path $exe -PathType Leaf)) { throw "Missing satellite host executable at $exe" }
$manifest = Join-Path $folder "$hostName.json"
@{
    name = $hostName
    description = 'Meeting Recorder browser satellite (current user only)'
    path = $exe
    type = 'stdio'
    allowed_origins = @("chrome-extension://$ExtensionId/")
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifest -Encoding UTF8
# PowerShell 5.1 Set-Content -Encoding UTF8 adds a BOM. Chromium's JSON loader
# generally accepts BOMs, but generate canonical UTF8 *without* BOM.
$json = Get-Content -Raw -LiteralPath $manifest
[System.IO.File]::WriteAllText($manifest, $json, (New-Object System.Text.UTF8Encoding($false)))
if ($Browser -in @('Chrome','Both')) {
    $path = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$hostName"
    New-Item $path -Force | Out-Null
    Set-Item -Path $path -Value $manifest
}
if ($Browser -in @('Edge','Both')) {
    $path = "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$hostName"
    New-Item $path -Force | Out-Null
    Set-Item -Path $path -Value $manifest
}
Write-Host "Native Messaging registered for $Browser (user only)."
Write-Host "Extension: $ExtensionId"
Write-Host "Manifest: $manifest"
Write-Host 'No recording was started and no app was launched.'
