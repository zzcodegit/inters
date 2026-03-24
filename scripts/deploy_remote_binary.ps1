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

function Deploy-Node(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$HostKey,
    [string]$Service,
    [string]$LocalBinary,
    [string]$RemoteBinary
) {
    $remote = "$User@$RemoteHost"
    $remoteTmp = "/tmp/vpnnode-$Label.new"
    $remoteDir = Get-RemoteDir $RemoteBinary
    $sshArgs = New-SshArgs -Password $Password -HostKey $HostKey
    Invoke-CheckedExternal -FailureMessage "failed to copy binary to $Label ($RemoteHost)" -Action {
        & $pscp @sshArgs $LocalBinary "${remote}:${remoteTmp}"
    }
    $command = @"
install -d -m 755 "$remoteDir"
install -D -m 755 "$remoteTmp" "$RemoteBinary"
rm -f "$remoteTmp"
systemctl restart $Service
systemctl is-active $Service
"@
    Invoke-CheckedExternal -FailureMessage "failed to install/restart $Service on $Label ($RemoteHost)" -Action {
        & $plink @sshArgs $remote $command
    }
    Write-Host "deployed label=$Label host=$RemoteHost service=$Service remote_binary=$RemoteBinary"
}

$localBinary = Get-OptionalEnv "VPNNODE_DEPLOY_BINARY_PATH" "$(Join-Path (Get-Location) 'target\release\vpnnode.exe')"
if (-not (Test-Path $localBinary)) {
    throw "Local binary not found at $localBinary"
}

$remoteBinary = Get-OptionalEnv "VPNNODE_REMOTE_BINARY_PATH" "/opt/vpnnode/bin/vpnnode"
$deployLabels = Get-OptionalCsvEnv "VPNNODE_REMOTE_DEPLOY_LABELS" @("relay1", "relay2", "exit")

if ($deployLabels -contains "relay1") {
    Deploy-Node -Label "relay-1" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_HOSTKEY" "") `
        -Service "vpnnode-relay1.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "relay2") {
    Deploy-Node -Label "relay-2" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_HOSTKEY" "") `
        -Service "vpnnode-relay2.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "relay3") {
    Deploy-Node -Label "relay-3" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_HOSTKEY" "") `
        -Service "vpnnode-relay3.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "relay4") {
    Deploy-Node -Label "relay-4" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_HOSTKEY" "") `
        -Service "vpnnode-relay4.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "relay5") {
    Deploy-Node -Label "relay-5" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_HOSTKEY" "") `
        -Service "vpnnode-relay5.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "relay6") {
    Deploy-Node -Label "relay-6" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_HOSTKEY" "") `
        -Service "vpnnode-relay6.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}

if ($deployLabels -contains "exit") {
    Deploy-Node -Label "exit" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_EXIT_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_EXIT_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_EXIT_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_EXIT_HOSTKEY" "") `
        -Service "vpnnode-exit.service" `
        -LocalBinary $localBinary `
        -RemoteBinary $remoteBinary
}
