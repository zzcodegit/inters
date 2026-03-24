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

function Invoke-CheckedExternal([scriptblock]$Action, [string]$FailureMessage) {
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit=$LASTEXITCODE)"
    }
}

$exitHost = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST"
$exitUser = Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root"
$exitPassword = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD"
$publicServiceName = Get-OptionalEnv "VPNNODE_PERF_PUBLIC_SERVICE" "vpnnode-target-http-public.service"

$remote = "$exitUser@$exitHost"
$command = @'
rm -f /etc/systemd/system/vpnnode-exit.service.d/20-stage-trace.conf
systemctl daemon-reload
systemctl restart vpnnode-exit.service
systemctl stop __PUBLIC_SERVICE__ || true
systemctl disable __PUBLIC_SERVICE__ || true
exit_status=$(systemctl is-active vpnnode-exit.service)
public_status=$(systemctl is-active __PUBLIC_SERVICE__ 2>/dev/null || true)
printf 'exit_service=%s\n' "$exit_status"
printf 'public_service=%s\n' "$public_status"
'@.Replace("__PUBLIC_SERVICE__", $publicServiceName)

Invoke-CheckedExternal -FailureMessage "failed to cleanup exit stage trace on $exitHost" -Action {
    & $plink -batch -pw $exitPassword $remote $command
}
Write-Host "cleanup_status=done"
