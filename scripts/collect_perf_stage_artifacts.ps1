param()

$ErrorActionPreference = "Stop"

$pscp = "C:\Program Files\PuTTY\pscp.exe"
if (-not (Test-Path $pscp)) {
    throw "pscp.exe not found at $pscp"
}

function Get-RequiredEnv([string]$Name) {
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "$Name must be set"
    }
    return $value
}

function Get-OptionalEnv([string]$Name, [string]$Default) {
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $Default
    }
    return $value
}

function Invoke-CheckedExternal([scriptblock]$Action, [string]$FailureMessage) {
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit=$LASTEXITCODE)"
    }
}

$exitHost = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST"
$exitUser = Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root"
$exitPassword = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD"
$exitStageLogPath = Get-OptionalEnv "VPNNODE_PERF_EXIT_STAGE_LOG_PATH" "/var/log/vpnnode/overlay-stage-exit.jsonl"
$forwardLogPath = Get-OptionalEnv "VPNNODE_PERF_FORWARD_LOG_PATH" "/var/log/vpnnode/perf-forward-stage.jsonl"

$localExitPath = Get-RequiredEnv "VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH"
$localDirectPath = Get-RequiredEnv "VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH"

$localExitDir = Split-Path -Parent $localExitPath
$localDirectDir = Split-Path -Parent $localDirectPath
if (-not [string]::IsNullOrWhiteSpace($localExitDir)) {
    New-Item -ItemType Directory -Force -Path $localExitDir | Out-Null
}
if (-not [string]::IsNullOrWhiteSpace($localDirectDir)) {
    New-Item -ItemType Directory -Force -Path $localDirectDir | Out-Null
}

$remote = "$exitUser@$exitHost"
Invoke-CheckedExternal -FailureMessage "failed to fetch exit stage log from $exitHost" -Action {
    & $pscp -batch -pw $exitPassword "${remote}:${exitStageLogPath}" $localExitPath
}
Invoke-CheckedExternal -FailureMessage "failed to fetch direct stage log from $exitHost" -Action {
    & $pscp -batch -pw $exitPassword "${remote}:${forwardLogPath}" $localDirectPath
}

Write-Host "collected_exit_stage_log=$localExitPath"
Write-Host "collected_direct_stage_log=$localDirectPath"
