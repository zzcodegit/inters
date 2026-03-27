param()

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "..\examples\config\remote_test_servers.ps1")

$env:VPNNODE_REMOTE_ACTIVE_LABELS = "relay1,relay2,relay3,relay4,relay6,exit"
$env:VPNNODE_REMOTE_DEPLOY_LABELS = "relay1,relay2,relay3,relay4,relay6,exit"
$env:VPNNODE_REMOTE_PROVISION_RELAYS = "relay1,relay2,relay3,relay4,relay6"

& powershell.exe -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "reset_remote_topology_singleline.ps1")
exit $LASTEXITCODE
