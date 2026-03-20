param()

$ErrorActionPreference = "Stop"

Set-Location -Path (Join-Path $PSScriptRoot "..")

$logPath = Join-Path (Get-Location) "baseline_ps_full.log"
if (Test-Path $logPath) {
  Remove-Item -Force $logPath
}

Start-Transcript -Path $logPath -Force | Out-Null

$env:CARGO_TERM_COLOR = "never"
$env:CARGO_TARGET_DIR = (Join-Path $env:TEMP "vpnnode-baseline-target")

if (!(Test-Path $env:CARGO_TARGET_DIR)) {
  New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR | Out-Null
}

try {
  for ($i = 1; $i -le 5; $i++) {
    Write-Host ("=== baseline run {0}/5 ===" -f $i)
    cargo test --tests baseline -- --test-threads=1
    if ($LASTEXITCODE -ne 0) {
      exit $LASTEXITCODE
    }
  }
}
finally {
  Stop-Transcript | Out-Null
}

