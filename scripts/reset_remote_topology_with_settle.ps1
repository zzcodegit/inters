param()

$ErrorActionPreference = "Stop"

& powershell -ExecutionPolicy Bypass -File "$PSScriptRoot\reset_remote_topology_singleline.ps1"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
