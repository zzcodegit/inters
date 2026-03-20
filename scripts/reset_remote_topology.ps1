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

function Restart-RemoteService(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string[]]$Services
) {
    $restartList = ($Services | ForEach-Object { $_.Trim() }) -join " "
    $statusChecks = $Services | ForEach-Object { "systemctl is-active $_" }
    $command = @(
        "systemctl restart $restartList"
        $statusChecks
    ) -join " && "

    Write-Host "reset_topology label=$Label host=$RemoteHost services=$restartList"
    & $plink -batch -pw $Password "$User@$RemoteHost" $command
}

function Assert-RemoteServiceActive(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$Service
) {
    Write-Host "check_service label=$Label host=$RemoteHost service=$Service"
    & $plink -batch -pw $Password "$User@$RemoteHost" "systemctl is-active $Service"
}

$relay1Host = Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_HOST"
$relay1User = Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_USER" "root"
$relay1Password = Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_PASSWORD"

$relay2Host = Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_HOST"
$relay2User = Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_USER" "root"
$relay2Password = Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_PASSWORD"

$exitHost = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST"
$exitUser = Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root"
$exitPassword = Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD"

Restart-RemoteService -Label "relay-1" -RemoteHost $relay1Host -User $relay1User -Password $relay1Password -Services @("vpnnode-relay1.service")
Restart-RemoteService -Label "relay-2" -RemoteHost $relay2Host -User $relay2User -Password $relay2Password -Services @("vpnnode-relay2.service")
Restart-RemoteService -Label "exit" -RemoteHost $exitHost -User $exitUser -Password $exitPassword -Services @("vpnnode-exit.service")
Assert-RemoteServiceActive -Label "exit-target-http" -RemoteHost $exitHost -User $exitUser -Password $exitPassword -Service "vpnnode-target-http.service"
