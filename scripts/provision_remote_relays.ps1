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

function Provision-Relay(
    [string]$Label,
    [string]$RemoteHost,
    [string]$User,
    [string]$Password,
    [string]$HostKey,
    [string]$ConfigName,
    [string]$ServiceName,
    [string]$ListenPort,
    [string]$DownstreamAddr
) {
    $remote = "$User@$RemoteHost"
    $sshArgs = New-SshArgs -Password $Password -HostKey $HostKey
    $downstreamParts = $DownstreamAddr.Split(":")
    if ($downstreamParts.Count -ne 2) {
        throw "DownstreamAddr must be host:port, got $DownstreamAddr"
    }
    $downstreamHost = $downstreamParts[0]
    $downstreamPort = $downstreamParts[1]
    $configContent = @"
role = "relay"
bind_ip = "0.0.0.0"
bind_port = $ListenPort
peers = [
  { ip = "$downstreamHost", port = $downstreamPort, protocol = "Udp" },
]
ants_enabled = false
drain_timeout_sec = 20
health_enabled = true
"@
    $unitContent = @"
[Unit]
Description=vpnnode $Label node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/opt/vpnnode/bin/vpnnode run --config /etc/vpnnode/$ConfigName
WorkingDirectory=/opt/vpnnode
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"@
    $configB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($configContent))
    $unitB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($unitContent))
    $command = @'
install -d -m 755 /etc/vpnnode /opt/vpnnode /opt/vpnnode/bin
printf '%s' '__CONFIG_B64__' | base64 -d > /etc/vpnnode/__CONFIG_NAME__
printf '%s' '__UNIT_B64__' | base64 -d > /etc/systemd/system/__SERVICE_NAME__
systemctl daemon-reload
systemctl enable __SERVICE_NAME__
systemctl restart __SERVICE_NAME__
systemctl is-active __SERVICE_NAME__
'@
    $command = $command `
        -replace '__CONFIG_NAME__', $ConfigName `
        -replace '__SERVICE_NAME__', $ServiceName `
        -replace '__CONFIG_B64__', $configB64 `
        -replace '__UNIT_B64__', $unitB64
    Invoke-CheckedExternal -FailureMessage "failed to provision $Label on $RemoteHost" -Action {
        & $plink @sshArgs $remote $command
    }
    Write-Host "provisioned label=$Label host=$RemoteHost service=$ServiceName listen_port=$ListenPort downstream=$DownstreamAddr"
}

$labels = Get-OptionalCsvEnv "VPNNODE_REMOTE_PROVISION_RELAYS" @("relay3", "relay4")

if ($labels -contains "relay3") {
    Provision-Relay -Label "relay-3" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_HOSTKEY" "") `
        -ConfigName "relay3.toml" `
        -ServiceName "vpnnode-relay3.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_PORT" "30003") `
        -DownstreamAddr (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_DOWNSTREAM" (Get-RequiredEnv "VPNNODE_BASELINE_REMOTE_EXIT_ADDR"))
}

if ($labels -contains "relay4") {
    Provision-Relay -Label "relay-4" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_HOSTKEY" "") `
        -ConfigName "relay4.toml" `
        -ServiceName "vpnnode-relay4.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_PORT" "30004") `
        -DownstreamAddr (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_DOWNSTREAM" (Get-RequiredEnv "VPNNODE_BASELINE_REMOTE_EXIT_ADDR"))
}

if ($labels -contains "relay5") {
    Provision-Relay -Label "relay-5" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_HOSTKEY" "") `
        -ConfigName "relay5.toml" `
        -ServiceName "vpnnode-relay5.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_PORT" "30005") `
        -DownstreamAddr (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_DOWNSTREAM" (Get-RequiredEnv "VPNNODE_BASELINE_REMOTE_EXIT_ADDR"))
}
