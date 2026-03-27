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
    [string]$ListenPort
) {
    $remote = "$User@$RemoteHost"
    $sshArgs = New-SshArgs -Password $Password -HostKey $HostKey
    $configContent = @"
role = "relay"
bind_ip = "0.0.0.0"
bind_port = $ListenPort
peers = []
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
    $command = @(
        'install -d -m 755 /etc/vpnnode /opt/vpnnode /opt/vpnnode/bin',
        "printf '%s' '$configB64' | base64 -d > /etc/vpnnode/$ConfigName",
        "printf '%s' '$unitB64' | base64 -d > /etc/systemd/system/$ServiceName",
        "systemctl daemon-reload",
        "systemctl enable $ServiceName",
        "systemctl restart $ServiceName",
        "systemctl is-active $ServiceName"
    ) -join "; "
    Invoke-CheckedExternal -FailureMessage "failed to provision $Label on $RemoteHost" -Action {
        & $plink @sshArgs $remote $command
    }
    Write-Host "provisioned label=$Label host=$RemoteHost service=$ServiceName listen_port=$ListenPort exact_route_mode=header_driven"
}

$labels = Get-OptionalCsvEnv "VPNNODE_REMOTE_PROVISION_RELAYS" @("relay1", "relay2", "relay3", "relay4", "relay5", "relay6")

if ($labels -contains "relay1") {
    Provision-Relay -Label "relay-1" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY1_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_HOSTKEY" "") `
        -ConfigName "relay1.toml" `
        -ServiceName "vpnnode-relay1.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY1_PORT" "30001")
}

if ($labels -contains "relay2") {
    Provision-Relay -Label "relay-2" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY2_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_HOSTKEY" "") `
        -ConfigName "relay2.toml" `
        -ServiceName "vpnnode-relay2.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY2_PORT" "30002")
}

if ($labels -contains "relay3") {
    Provision-Relay -Label "relay-3" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY3_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_HOSTKEY" "") `
        -ConfigName "relay3.toml" `
        -ServiceName "vpnnode-relay3.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY3_PORT" "30003")
}

if ($labels -contains "relay4") {
    Provision-Relay -Label "relay-4" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY4_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_HOSTKEY" "") `
        -ConfigName "relay4.toml" `
        -ServiceName "vpnnode-relay4.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY4_PORT" "30004")
}

if ($labels -contains "relay5") {
    Provision-Relay -Label "relay-5" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY5_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_HOSTKEY" "") `
        -ConfigName "relay5.toml" `
        -ServiceName "vpnnode-relay5.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY5_PORT" "30005")
}

if ($labels -contains "relay6") {
    Provision-Relay -Label "relay-6" `
        -RemoteHost (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_HOST") `
        -User (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_USER" "root") `
        -Password (Get-RequiredEnv "VPNNODE_REMOTE_RELAY6_PASSWORD") `
        -HostKey (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_HOSTKEY" "") `
        -ConfigName "relay6.toml" `
        -ServiceName "vpnnode-relay6.service" `
        -ListenPort (Get-OptionalEnv "VPNNODE_REMOTE_RELAY6_PORT" "30006")
}
