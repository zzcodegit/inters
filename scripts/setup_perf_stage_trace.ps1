param()

$ErrorActionPreference = "Stop"

$plink = "C:\Program Files\PuTTY\plink.exe"
if (-not (Test-Path $plink)) {
    throw "plink.exe not found at $plink"
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

function Get-RemoteDir([string]$RemotePath) {
    $lastSlash = $RemotePath.LastIndexOf("/")
    if ($lastSlash -lt 0) {
        return "."
    }
    if ($lastSlash -eq 0) {
        return "/"
    }
    return $RemotePath.Substring(0, $lastSlash)
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
$exitStageLogDir = Get-RemoteDir $exitStageLogPath

$remote = "$exitUser@$exitHost"
$command = @"
mkdir -p /etc/systemd/system/vpnnode-exit.service.d
mkdir -p "$exitStageLogDir"
: > "$exitStageLogPath"
cat > /etc/systemd/system/vpnnode-exit.service.d/20-stage-trace.conf <<'UNIT'
[Service]
Environment=VPNNODE_STAGE_TRACE_PATH=$exitStageLogPath
UNIT
systemctl daemon-reload
systemctl restart vpnnode-exit.service
systemctl is-active vpnnode-exit.service
"@

Invoke-CheckedExternal -FailureMessage "failed to enable exit stage trace on $exitHost" -Action {
    & $plink -batch -pw $exitPassword $remote $command
}
Write-Host "exit_stage_log_path=$exitStageLogPath"
