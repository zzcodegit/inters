param(
  [switch]$FixedPorts
)

# Native run (no Docker) to isolate networking issues.
# Run from vpnnode directory.
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$bin = "target\release\vpnnode.exe"
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$env:RUST_LOG = "debug"

# Use fixed ports (to match checklist) or random high ports (to avoid conflicts)
if ($FixedPorts) {
  $targetPort = 8080
  $exitPort = 30001
  $relayPort = 30000
  $clientPort = 10080
  Write-Host "Using FIXED ports: target=$targetPort exit=$exitPort relay=$relayPort client=$clientPort"
} else {
  $base = 40000 + (Get-Random -Maximum 10000)
  $targetPort = $base
  $exitPort = $base + 1
  $relayPort = $base + 2
  $clientPort = $base + 3
  Write-Host "Using RANDOM ports: target=$targetPort exit=$exitPort relay=$relayPort client=$clientPort"
  Write-Host "Tip: run with -FixedPorts to use 8080/30001/30000/10080 for curl http://127.0.0.1:10080/"
}

Get-Process -Name vpnnode -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3

$logDir = "run-local-logs"
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

$target = Start-Process -FilePath $bin -ArgumentList "target","--listen","0.0.0.0:$targetPort" -PassThru -RedirectStandardOutput "$logDir\target.out" -RedirectStandardError "$logDir\target.err"
Start-Sleep -Milliseconds 500
$exit   = Start-Process -FilePath $bin -ArgumentList "exit","--listen","0.0.0.0:$exitPort","--target-addr","127.0.0.1:$targetPort" -PassThru -RedirectStandardOutput "$logDir\exit.out" -RedirectStandardError "$logDir\exit.err"
Start-Sleep -Milliseconds 500
$relay  = Start-Process -FilePath $bin -ArgumentList "relay","--listen","127.0.0.1:$relayPort","--exit-addr","127.0.0.1:$exitPort" -PassThru -RedirectStandardOutput "$logDir\relay.out" -RedirectStandardError "$logDir\relay.err"
Start-Sleep -Seconds 1
$client = Start-Process -FilePath $bin -ArgumentList "client","--local-listen","0.0.0.0:$clientPort","--relay-addr","127.0.0.1:$relayPort","--exit-addr","127.0.0.1:$exitPort","--route-length","2" -PassThru -RedirectStandardOutput "$logDir\client.out" -RedirectStandardError "$logDir\client.err"
Start-Sleep -Seconds 4

Write-Host "Running curl to http://127.0.0.1:$clientPort/ ..."
try {
  $r = curl.exe -s -w "\n%{http_code}" -m 15 "http://127.0.0.1:$clientPort/" 2>&1
  Write-Host "Response: $r"
} catch {
  Write-Host "Curl error: $_"
}

Start-Sleep -Seconds 2
Stop-Process -Id $client.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $relay.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $exit.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $target.Id -Force -ErrorAction SilentlyContinue

Write-Host "`n=== client err (last 30 lines) ==="
Get-Content "$logDir\client.err" -Tail 30 -ErrorAction SilentlyContinue
Write-Host "`n=== relay err (last 20 lines) ==="
Get-Content "$logDir\relay.err" -Tail 20 -ErrorAction SilentlyContinue
