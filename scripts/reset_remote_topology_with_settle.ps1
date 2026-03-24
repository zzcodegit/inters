param()

$ErrorActionPreference = "Stop"

$settleSeconds = [Environment]::GetEnvironmentVariable("VPNNODE_REMOTE_RESET_SETTLE_SECS")
if ([string]::IsNullOrWhiteSpace($settleSeconds)) {
    $settleSeconds = "15"
}

& powershell -ExecutionPolicy Bypass -File "$PSScriptRoot\reset_remote_topology.ps1"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$settle = 0
if (-not [int]::TryParse($settleSeconds, [ref]$settle)) {
    throw "VPNNODE_REMOTE_RESET_SETTLE_SECS must be an integer, got '$settleSeconds'"
}

Write-Host "reset_settle_secs=$settle"
if ($settle -gt 0) {
    Start-Sleep -Seconds $settle
}
