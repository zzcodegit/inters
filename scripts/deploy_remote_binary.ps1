param()

$ErrorActionPreference = "Stop"

$pscp = "C:\Program Files\PuTTY\pscp.exe"
$plink = "C:\Program Files\PuTTY\plink.exe"
if (-not (Test-Path $pscp)) {
    throw "pscp.exe not found at $pscp"
}
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

function Deploy-Node(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$Service,
    [string]$LocalBinary,
    [string]$RemoteBinary
) {
    $remote = "$User@$RemoteHost"
    $remoteTmp = "${RemoteBinary}.new"
    & $pscp -batch -pw $Password $LocalBinary "${remote}:${remoteTmp}"
    $command = @"
install -m 755 "$remoteTmp" "$RemoteBinary"
rm -f "$remoteTmp"
systemctl restart $Service
systemctl is-active $Service
"@
    & $plink -batch -pw $Password $remote $command
    Write-Host "deployed label=$Label host=$RemoteHost service=$Service remote_binary=$RemoteBinary"
}

$localBinary = Get-OptionalEnv "VPNNODE_DEPLOY_BINARY_PATH" "$(Join-Path (Get-Location) 'target\release\vpnnode.exe')"
if (-not (Test-Path $localBinary)) {
    throw "Local binary not found at $localBinary"
}

$remoteBinary = Get-OptionalEnv "VPNNODE_REMOTE_BINARY_PATH" "/opt/vpnnode/bin/vpnnode"

Deploy-Node -Label "relay-1" `
    -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_HOST") `
    -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_USER" "root") `
    -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_PASSWORD") `
    -Service "vpnnode-relay1.service" `
    -LocalBinary $localBinary `
    -RemoteBinary $remoteBinary

Deploy-Node -Label "relay-2" `
    -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_HOST") `
    -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_USER" "root") `
    -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_PASSWORD") `
    -Service "vpnnode-relay2.service" `
    -LocalBinary $localBinary `
    -RemoteBinary $remoteBinary

Deploy-Node -Label "exit" `
    -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST") `
    -User (Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root") `
    -Password (Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD") `
    -Service "vpnnode-exit.service" `
    -LocalBinary $localBinary `
    -RemoteBinary $remoteBinary
