$ErrorActionPreference = "Stop"

$runTag = $env:VPNNODE_CHAOS_RUN_TAG
if ([string]::IsNullOrWhiteSpace($runTag)) {
    $runTag = Get-Date -Format "yyyy-MM-dd"
}

$runPrefix = "docs/artifacts/chaos_matrix_$runTag"
$masterLog = "$runPrefix.log"
$cleanupLog = "${runPrefix}_cleanup.log"

$profiles = @(
    @{
        Name = "none"
        BasePort = 19300
        Seed = 1
        SkipPackets = 0
        LossPpm = 0
        DuplicatePpm = 0
        ReorderPpm = 0
        BaseDelayMs = 0
        JitterMs = 0
        ReorderExtraDelayMs = 0
        DuplicateDelayMs = 2
    },
    @{
        Name = "mild-loss"
        BasePort = 19400
        Seed = 17
        SkipPackets = 24
        LossPpm = 1000
        DuplicatePpm = 0
        ReorderPpm = 0
        BaseDelayMs = 0
        JitterMs = 0
        ReorderExtraDelayMs = 0
        DuplicateDelayMs = 2
    },
    @{
        Name = "mild-reorder"
        BasePort = 19500
        Seed = 23
        SkipPackets = 24
        LossPpm = 0
        DuplicatePpm = 0
        ReorderPpm = 20000
        BaseDelayMs = 0
        JitterMs = 0
        ReorderExtraDelayMs = 40
        DuplicateDelayMs = 2
    },
    @{
        Name = "mild-delay"
        BasePort = 19600
        Seed = 29
        SkipPackets = 24
        LossPpm = 0
        DuplicatePpm = 0
        ReorderPpm = 0
        BaseDelayMs = 4
        JitterMs = 2
        ReorderExtraDelayMs = 0
        DuplicateDelayMs = 2
    },
    @{
        Name = "combined"
        BasePort = 19700
        Seed = 31
        SkipPackets = 24
        LossPpm = 1000
        DuplicatePpm = 2000
        ReorderPpm = 15000
        BaseDelayMs = 8
        JitterMs = 6
        ReorderExtraDelayMs = 30
        DuplicateDelayMs = 3
    }
)

New-Item -ItemType Directory -Force -Path "docs/artifacts" | Out-Null
if (Test-Path $masterLog) {
    Remove-Item $masterLog -Force
}

$profileNames = @()
$failed = $false
$failureMessage = $null

try {
    foreach ($profile in $profiles) {
        $profileNames += $profile.Name
        $artifactPrefix = "${runPrefix}_$($profile.Name)"
        $env:VPNNODE_CHAOS_PROFILE_NAME = $profile.Name
        $env:VPNNODE_CHAOS_ARTIFACT_PREFIX = $artifactPrefix
        $env:VPNNODE_STAGE_TRACE_PATH = "${artifactPrefix}.stage.jsonl"
        $env:VPNNODE_CHAOS_BASE_PORT = [string]$profile.BasePort
        $env:VPNNODE_CHAOS_RUNS = "5"
        $env:VPNNODE_CHAOS_REQUEST_BODY_BYTES = "32768"
        $env:VPNNODE_TRANSPORT_CHAOS_LABEL = $profile.Name
        $env:VPNNODE_TRANSPORT_CHAOS_SEED = [string]$profile.Seed
        $env:VPNNODE_TRANSPORT_CHAOS_SKIP_PACKETS = [string]$profile.SkipPackets
        $env:VPNNODE_TRANSPORT_CHAOS_LOSS_PPM = [string]$profile.LossPpm
        $env:VPNNODE_TRANSPORT_CHAOS_DUPLICATE_PPM = [string]$profile.DuplicatePpm
        $env:VPNNODE_TRANSPORT_CHAOS_REORDER_PPM = [string]$profile.ReorderPpm
        $env:VPNNODE_TRANSPORT_CHAOS_BASE_DELAY_MS = [string]$profile.BaseDelayMs
        $env:VPNNODE_TRANSPORT_CHAOS_JITTER_MS = [string]$profile.JitterMs
        $env:VPNNODE_TRANSPORT_CHAOS_REORDER_EXTRA_DELAY_MS = [string]$profile.ReorderExtraDelayMs
        $env:VPNNODE_TRANSPORT_CHAOS_DUPLICATE_DELAY_MS = [string]$profile.DuplicateDelayMs

        "=== chaos profile=$($profile.Name) base_port=$($profile.BasePort) loss_ppm=$($profile.LossPpm) duplicate_ppm=$($profile.DuplicatePpm) reorder_ppm=$($profile.ReorderPpm) base_delay_ms=$($profile.BaseDelayMs) jitter_ms=$($profile.JitterMs) reorder_extra_delay_ms=$($profile.ReorderExtraDelayMs) ===" |
            Tee-Object -FilePath $masterLog -Append

        $cargoCommand = "cargo test --test chaos_matrix chaos_profile_collects_transport_and_selection_artifacts -- --test-threads=1 --nocapture"
        $stdoutPath = Join-Path $env:TEMP "vpnnode-chaos-$($profile.Name)-stdout.log"
        $stderrPath = Join-Path $env:TEMP "vpnnode-chaos-$($profile.Name)-stderr.log"
        if (Test-Path $stdoutPath) { Remove-Item $stdoutPath -Force }
        if (Test-Path $stderrPath) { Remove-Item $stderrPath -Force }
        $process = Start-Process -FilePath "cmd.exe" `
            -ArgumentList "/c", $cargoCommand `
            -WorkingDirectory (Get-Location).Path `
            -NoNewWindow `
            -Wait `
            -PassThru `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath
        if (Test-Path $stdoutPath) {
            Get-Content $stdoutPath | Tee-Object -FilePath $masterLog -Append
        }
        if (Test-Path $stderrPath) {
            Get-Content $stderrPath | Tee-Object -FilePath $masterLog -Append
        }
        if ($process.ExitCode -ne 0) {
            $failed = $true
            $failureMessage = "chaos profile '$($profile.Name)' failed with exit code $($process.ExitCode)"
            break
        }
    }
}
finally {
    if ($profileNames.Count -gt 0 -and (Test-Path "${runPrefix}_$($profileNames[0]).profile.json")) {
        python scripts/summarize_chaos_matrix.py --run-prefix $runPrefix --profiles $profileNames
    }

    "cleanup run_prefix=$runPrefix" | Out-File -FilePath $cleanupLog -Encoding utf8
    $ports = @()
    foreach ($profile in $profiles) {
        $basePort = [int]$profile.BasePort
        $ports += ($basePort + 1)
        $ports += ($basePort + 2)
        $ports += ($basePort + 3)
        $ports += ($basePort + 4)
        $ports += ($basePort + 80)
    }
    "ports=$($ports -join ',')" | Out-File -FilePath $cleanupLog -Encoding utf8 -Append
    "processes:" | Out-File -FilePath $cleanupLog -Encoding utf8 -Append
    (Get-Process | Where-Object { $_.ProcessName -match 'vpnnode|cargo|rustc' } |
        Select-Object Id, ProcessName, Path |
        Format-Table -AutoSize | Out-String) | Out-File -FilePath $cleanupLog -Encoding utf8 -Append
    "netstat:" | Out-File -FilePath $cleanupLog -Encoding utf8 -Append
    $pattern = ($ports | ForEach-Object { ":{0}" -f $_ }) -join "|"
    ((netstat -ano) | Select-String -Pattern $pattern | Out-String) | Out-File -FilePath $cleanupLog -Encoding utf8 -Append

    Remove-Item Env:VPNNODE_CHAOS_PROFILE_NAME -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_CHAOS_ARTIFACT_PREFIX -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_STAGE_TRACE_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_CHAOS_BASE_PORT -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_CHAOS_RUNS -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_CHAOS_REQUEST_BODY_BYTES -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_LABEL -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_SEED -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_SKIP_PACKETS -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_LOSS_PPM -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_DUPLICATE_PPM -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_REORDER_PPM -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_BASE_DELAY_MS -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_JITTER_MS -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_REORDER_EXTRA_DELAY_MS -ErrorAction SilentlyContinue
    Remove-Item Env:VPNNODE_TRANSPORT_CHAOS_DUPLICATE_DELAY_MS -ErrorAction SilentlyContinue
}

if ($failed) {
    Write-Error $failureMessage
    exit 1
}
