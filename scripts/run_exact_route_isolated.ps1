param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("1", "2", "3", "5")]
    [string]$RouteLength,

    [Parameter(Mandatory = $true)]
    [string]$DateTag
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

. (Join-Path $PSScriptRoot "..\examples\config\remote_test_servers.ps1")

$env:VPNNODE_REMOTE_ACTIVE_LABELS = "relay1,relay2,relay3,relay4,relay6,exit"
$env:VPNNODE_REMOTE_DEPLOY_LABELS = "relay1,relay2,relay3,relay4,relay6,exit"
$env:VPNNODE_REMOTE_PROVISION_RELAYS = "relay1,relay2,relay3,relay4,relay6"

$routeName = "${RouteLength}hop"
$artifactsDir = Join-Path $PSScriptRoot "..\docs\artifacts"
$resetLog = Join-Path $artifactsDir "stage5_baseline_v2_finalize_routeproof_reset_${routeName}_${DateTag}.log"
$runLog = Join-Path $artifactsDir "stage5_baseline_v2_finalize_routeproof_run_${routeName}_${DateTag}.log"
$journalSinceEpoch = [DateTimeOffset]::new((Get-Date).ToUniversalTime().AddSeconds(-5)).ToUnixTimeSeconds()

$cachePaths = @(
    "route_cache_remote_${routeName}.json",
    "route_cache_remote_baseline.json"
)

foreach ($cachePath in $cachePaths) {
    Remove-Item $cachePath -Force -ErrorAction SilentlyContinue
}

& powershell.exe -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "reset_remote_topology_singleline.ps1") *>&1 |
    Tee-Object $resetLog
if ($LASTEXITCODE -ne 0) {
    throw "reset failed for $routeName (exit=$LASTEXITCODE)"
}

$testName = "remote_http_${routeName}_returns_200"
$cargoArgs = "/c cargo test --test remote_baseline -- --exact $testName --nocapture > `"$runLog`" 2>&1"
& cmd.exe $cargoArgs
$testExit = $LASTEXITCODE
Get-Content $runLog

function New-SshArgs([string]$Password, [string]$HostKey) {
    $args = @("-batch")
    if (-not [string]::IsNullOrWhiteSpace($HostKey)) {
        $args += @("-hostkey", $HostKey)
    }
    $args += @("-pw", $Password)
    return $args
}

$plink = "C:\Program Files\PuTTY\plink.exe"
if (-not (Test-Path $plink)) {
    throw "plink.exe not found at $plink"
}

$nodeSpecs = @(
    @{ Label = "exit"; Host = $env:VPNNODE_REMOTE_EXIT_HOST; User = $env:VPNNODE_REMOTE_EXIT_USER; Password = $env:VPNNODE_REMOTE_EXIT_PASSWORD; HostKey = $env:VPNNODE_REMOTE_EXIT_HOSTKEY; Service = "vpnnode-exit.service" },
    @{ Label = "relay1"; Host = $env:VPNNODE_REMOTE_RELAY1_HOST; User = $env:VPNNODE_REMOTE_RELAY1_USER; Password = $env:VPNNODE_REMOTE_RELAY1_PASSWORD; HostKey = $env:VPNNODE_REMOTE_RELAY1_HOSTKEY; Service = "vpnnode-relay1.service" },
    @{ Label = "relay2"; Host = $env:VPNNODE_REMOTE_RELAY2_HOST; User = $env:VPNNODE_REMOTE_RELAY2_USER; Password = $env:VPNNODE_REMOTE_RELAY2_PASSWORD; HostKey = $env:VPNNODE_REMOTE_RELAY2_HOSTKEY; Service = "vpnnode-relay2.service" },
    @{ Label = "relay3"; Host = $env:VPNNODE_REMOTE_RELAY3_HOST; User = $env:VPNNODE_REMOTE_RELAY3_USER; Password = $env:VPNNODE_REMOTE_RELAY3_PASSWORD; HostKey = $env:VPNNODE_REMOTE_RELAY3_HOSTKEY; Service = "vpnnode-relay3.service" },
    @{ Label = "relay4"; Host = $env:VPNNODE_REMOTE_RELAY4_HOST; User = $env:VPNNODE_REMOTE_RELAY4_USER; Password = $env:VPNNODE_REMOTE_RELAY4_PASSWORD; HostKey = $env:VPNNODE_REMOTE_RELAY4_HOSTKEY; Service = "vpnnode-relay4.service" },
    @{ Label = "relay6"; Host = $env:VPNNODE_REMOTE_RELAY6_HOST; User = $env:VPNNODE_REMOTE_RELAY6_USER; Password = $env:VPNNODE_REMOTE_RELAY6_PASSWORD; HostKey = $env:VPNNODE_REMOTE_RELAY6_HOSTKEY; Service = "vpnnode-relay6.service" }
)

foreach ($node in $nodeSpecs) {
    $label = [string]$node["Label"]
    $remoteHost = [string]$node["Host"]
    $remoteUser = [string]$node["User"]
    $remotePassword = [string]$node["Password"]
    $remoteHostKey = [string]$node["HostKey"]
    $service = [string]$node["Service"]
    $journalLog = Join-Path $artifactsDir "stage5_baseline_v2_finalize_routeproof_${routeName}_${label}_journal_${DateTag}.log"
    $sshArgs = New-SshArgs -Password $remotePassword -HostKey $remoteHostKey
    & $plink @sshArgs "$remoteUser@$remoteHost" "journalctl -u $service --since=@$journalSinceEpoch --no-pager" *>&1 |
        Tee-Object $journalLog
    if ($LASTEXITCODE -ne 0) {
        throw "journal capture failed for $label after $routeName (exit=$LASTEXITCODE)"
    }
}

exit $testExit
