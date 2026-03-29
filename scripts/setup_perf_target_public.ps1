param()

$ErrorActionPreference = "Stop"

$plink = "C:\Program Files\PuTTY\plink.exe"
$pscp = "C:\Program Files\PuTTY\pscp.exe"
if (-not (Test-Path $plink)) {
    throw "plink.exe not found at $plink"
}
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

$targetDir = Get-OptionalEnv "VPNNODE_PERF_TARGET_DIR" "/opt/vpnnode/target-http"
$targetFileName = Get-OptionalEnv "VPNNODE_PERF_TARGET_FILE" "perf-262144.bin"
$targetBytes = Get-OptionalEnv "VPNNODE_PERF_TARGET_BYTES" "262144"
$publicPort = Get-OptionalEnv "VPNNODE_PERF_PUBLIC_PORT" "18080"
$publicScriptPath = Get-OptionalEnv "VPNNODE_PERF_FORWARD_SCRIPT_PATH" "/opt/vpnnode/bin/vpnnode-tcp-forward.py"
$publicServiceName = Get-OptionalEnv "VPNNODE_PERF_PUBLIC_SERVICE" "vpnnode-target-http-public.service"
$forwardLogPath = Get-OptionalEnv "VPNNODE_PERF_FORWARD_LOG_PATH" ""

$remote = "$exitUser@$exitHost"
$localForwardScript = Join-Path (Get-Location) "scripts\tcp_forward.py"
if (-not (Test-Path $localForwardScript)) {
    throw "Missing local forward script at $localForwardScript"
}

$payloadScript = @"
from pathlib import Path
target_dir = Path(r"$targetDir")
target_dir.mkdir(parents=True, exist_ok=True)
payload = (b"vpnnode-perf-" * 30000)[:int("$targetBytes")]
path = target_dir / "$targetFileName"
path.write_bytes(payload)
print(f"payload_path={path}")
print(f"payload_bytes={path.stat().st_size}")
"@
$payloadScriptB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($payloadScript))
$payloadCommand = "mkdir -p /opt/vpnnode/bin '$targetDir'; printf '%s' '$payloadScriptB64' | base64 -d | python3 -"

Invoke-CheckedExternal -FailureMessage "failed to prepare perf payload on $exitHost" -Action {
    & $plink -batch -pw $exitPassword $remote $payloadCommand
}
Invoke-CheckedExternal -FailureMessage "failed to upload tcp forward helper to $exitHost" -Action {
    & $pscp -batch -pw $exitPassword $localForwardScript "${remote}:${publicScriptPath}"
}

$forwardExec = "/usr/bin/python3 $publicScriptPath --listen-host 0.0.0.0 --listen-port $publicPort --target-host 127.0.0.1 --target-port 8080"
if (-not [string]::IsNullOrWhiteSpace($forwardLogPath)) {
    $forwardExec += " --stage-log-path $forwardLogPath"
}

$unitContent = @"
[Unit]
Description=Public TCP forward to vpnnode target-http
After=network.target vpnnode-target-http.service

[Service]
ExecStart=$forwardExec
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
"@
$unitContentB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($unitContent))
$unitCommandParts = @(
    "printf '%s' '$unitContentB64' | base64 -d > /etc/systemd/system/$publicServiceName",
    "chmod 755 $publicScriptPath"
)
if (-not [string]::IsNullOrWhiteSpace($forwardLogPath)) {
    $forwardLogDir = Split-Path -Parent $forwardLogPath
    if ([string]::IsNullOrWhiteSpace($forwardLogDir)) {
        $forwardLogDir = "/var/log/vpnnode"
    }
    $unitCommandParts += "mkdir -p '$forwardLogDir'"
    $unitCommandParts += ": > '$forwardLogPath'"
}
$unitCommandParts += @(
    "systemctl daemon-reload",
    "systemctl enable --now $publicServiceName",
    "systemctl is-active $publicServiceName"
)
$unitCommand = $unitCommandParts -join "; "

Invoke-CheckedExternal -FailureMessage "failed to configure public perf forwarder on $exitHost" -Action {
    & $plink -batch -pw $exitPassword $remote $unitCommand
}

Write-Host "direct_addr=${exitHost}:$publicPort"
Write-Host "target_path=/$targetFileName"
Write-Host "payload_bytes=$targetBytes"
Write-Host "public_service=$publicServiceName"
if (-not [string]::IsNullOrWhiteSpace($forwardLogPath)) {
    Write-Host "forward_log_path=$forwardLogPath"
}
