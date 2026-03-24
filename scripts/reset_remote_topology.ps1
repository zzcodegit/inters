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

function Get-OptionalCsvEnv([string]$Name, [string[]]$Default) {
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $Default
    }
    return $value.Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ }
}

function New-SshArgs([string]$Password, [string]$HostKey) {
    $args = @("-batch")
    if (-not [string]::IsNullOrWhiteSpace($HostKey)) {
        $args += @("-hostkey", $HostKey)
    }
    $args += @("-pw", $Password)
    return $args
}

function Invoke-CheckedExternal([scriptblock]$Action, [string]$FailureMessage) {
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit=$LASTEXITCODE)"
    }
}

function Restart-RemoteService(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$HostKey,
    [string[]]$Services
) {
    $restartList = ($Services | ForEach-Object { $_.Trim() }) -join " "
    $statusChecks = $Services | ForEach-Object {
        @'
for _ in 1 2 3 4 5 6 7 8 9 10; do
  state=$(systemctl is-active __SERVICE__ 2>/dev/null || true)
  if [ "$state" = "active" ]; then
    echo active
    break
  fi
  sleep 1
done
[ "$(systemctl is-active __SERVICE__ 2>/dev/null || true)" = "active" ]
'@ -replace '__SERVICE__', $_
    }
    $command = @(
        "systemctl restart $restartList"
        $statusChecks
    ) -join "`n"

    Write-Host "reset_topology label=$Label host=$RemoteHost services=$restartList"
    $sshArgs = New-SshArgs -Password $Password -HostKey $HostKey
    Invoke-CheckedExternal -FailureMessage "failed to restart/verify $restartList on $Label ($RemoteHost)" -Action {
        & $plink @sshArgs "$User@$RemoteHost" $command
    }
}

function Assert-RemoteServiceActive(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$HostKey,
    [string]$Service
) {
    Write-Host "check_service label=$Label host=$RemoteHost service=$Service"
    $sshArgs = New-SshArgs -Password $Password -HostKey $HostKey
    Invoke-CheckedExternal -FailureMessage "service $Service is not active on $Label ($RemoteHost)" -Action {
        $command = @'
for _ in 1 2 3 4 5 6 7 8 9 10; do
  state=$(systemctl is-active __SERVICE__ 2>/dev/null || true)
  if [ "$state" = "active" ]; then
    echo active
    break
  fi
  sleep 1
done
[ "$(systemctl is-active __SERVICE__ 2>/dev/null || true)" = "active" ]
'@ -replace '__SERVICE__', $Service
        & $plink @sshArgs "$User@$RemoteHost" $command
    }
}

$activeLabels = Get-OptionalCsvEnv "VPNNODE_REMOTE_ACTIVE_LABELS" @("relay1", "relay2", "exit")

if ($activeLabels -contains "relay1") {
    Restart-RemoteService -Label "relay-1" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_HOSTKEY" "") `
        -Services @("vpnnode-relay1.service")
}

if ($activeLabels -contains "relay2") {
    Restart-RemoteService -Label "relay-2" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_HOSTKEY" "") `
        -Services @("vpnnode-relay2.service")
}

if ($activeLabels -contains "relay3") {
    Restart-RemoteService -Label "relay-3" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_HOSTKEY" "") `
        -Services @("vpnnode-relay3.service")
}

if ($activeLabels -contains "relay4") {
    Restart-RemoteService -Label "relay-4" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_HOSTKEY" "") `
        -Services @("vpnnode-relay4.service")
}

if ($activeLabels -contains "relay5") {
    Restart-RemoteService -Label "relay-5" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_HOSTKEY" "") `
        -Services @("vpnnode-relay5.service")
}

if ($activeLabels -contains "relay6") {
    Restart-RemoteService -Label "relay-6" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_HOSTKEY" "") `
        -Services @("vpnnode-relay6.service")
}

$exitHost = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST"
$exitUser = Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root"
$exitPassword = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD"
$exitHostKey = Get-OptionalEnv "VPNNODE_REMOTE_EXIT_HOSTKEY" ""

if ($activeLabels -contains "exit") {
    Restart-RemoteService -Label "exit" -RemoteHost $exitHost -User $exitUser -Password $exitPassword -HostKey $exitHostKey -Services @("vpnnode-exit.service")
}
Assert-RemoteServiceActive -Label "exit-target-http" -RemoteHost $exitHost -User $exitUser -Password $exitPassword -HostKey $exitHostKey -Service "vpnnode-target-http.service"
