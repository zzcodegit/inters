param(
    [string[]]$Sites = @("wikipedia.org", "github.com", "cloudflare.com", "ozon.ru"),
    [string]$LocalOverlayHost = "127.0.0.1",
    [int]$LocalOverlayPort = 10080,

    [string]$UsaHost = "31.192.232.26",
    [int]$UsaPort = 30000,
    [string]$UsaSshUser = "root",
    [string]$UsaSshPassword = $env:USA_SSH_PASS,

    [string]$ExitUnit = "vpnnode-exit.service",

    [string]$ResultsCsv = ".\results_usa.csv",
    [string]$ArtifactLog = ".\artifacts_usa.log",
    [string]$RunLog = ".\run_usa_validation.log",

    [int]$CurlTimeout = 40,
    [int]$LogWaitMs = 1000,
    [string]$RemoteSinceWindow = "3 hours ago",

    [int]$RepeatsPerSite = 3
)

$ErrorActionPreference = "Stop"

$plinkPath = "C:\Program Files\PuTTY\plink.exe"

function Write-RunLog {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message
    # Suppress pipeline output from Tee-Object so callers don't accidentally capture it.
    $line | Tee-Object -FilePath $RunLog -Append | Out-Null
}

function Fail {
    param([string]$Message)
    Write-RunLog "ERROR: $Message"
    throw $Message
}

function Require-File {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        Fail "Missing required file: $Path"
    }
}

function Init-Files {
    "" | Set-Content -Path $RunLog -Encoding UTF8
    "" | Set-Content -Path $ArtifactLog -Encoding UTF8

    # validate_report.rs requires a subset of these columns; extra columns are OK.
    $header = "site,mode,run,exit,route,connect,appconnect,starttransfer,total,code,resp_bytes,frames_sent,avg_inflight,max_inflight,retransmit_rate,throughput_bps,ack_latency_ms_avg,artifact_raw"
    $header | Set-Content -Path $ResultsCsv -Encoding UTF8
}

function Csv-Escape {
    param([AllowNull()][string]$Value)
    if ($null -eq $Value) { $Value = "" }
    '"' + ($Value -replace '"', '""') + '"'
}

function Append-CsvRow {
    param(
        [string]$Site,
        [string]$Mode,
        [int]$Run,
        [string]$ExitField,
        [string]$Route,
        [string]$Connect,
        [string]$AppConnect,
        [string]$StartTransfer,
        [string]$Total,
        [string]$Code,
        [string]$RespBytes,
        [string]$FramesSent,
        [string]$AvgInflight,
        [string]$MaxInflight,
        [string]$RetransmitRate,
        [string]$ThroughputBps,
        [string]$AckLatencyMsAvg,
        [string]$ArtifactRaw
    )

    $fields = @(
        $Site, $Mode, $Run, $ExitField, $Route, $Connect, $AppConnect,
        $StartTransfer, $Total, $Code, $RespBytes, $FramesSent,
        $AvgInflight, $MaxInflight, $RetransmitRate, $ThroughputBps,
        $AckLatencyMsAvg, $ArtifactRaw
    )

    $line = ($fields | ForEach-Object { Csv-Escape $_ }) -join ","
    Add-Content -Path $ResultsCsv -Value $line -Encoding UTF8
}

function Parse-CurlMetrics {
    param([string]$Text)

    $result = @{
        connect = ""
        appconnect = ""
        starttransfer = ""
        total = ""
        code = ""
    }

    if ($Text -match 'connect=([0-9.]+)')       { $result.connect = $Matches[1] }
    if ($Text -match 'appconnect=([0-9.]+)')    { $result.appconnect = $Matches[1] }
    if ($Text -match 'starttransfer=([0-9.]+)') { $result.starttransfer = $Matches[1] }
    if ($Text -match 'total=([0-9.]+)')         { $result.total = $Matches[1] }
    if ($Text -match 'code=([0-9]+)')           { $result.code = $Matches[1] }

    return $result
}

function Parse-ArtifactField {
    param(
        [string]$Line,
        [string]$Key
    )

    if ([string]::IsNullOrWhiteSpace($Line)) { return "" }

    $pattern = [regex]::Escape($Key) + '=([^,]*)'
    $m = [regex]::Match($Line, $pattern)
    if ($m.Success) { return $m.Groups[1].Value }
    return ""
}

function Test-LocalOverlayListener {
    Write-RunLog "Checking local overlay listener at $LocalOverlayHost`:$LocalOverlayPort"
    $tcp = Test-NetConnection -ComputerName $LocalOverlayHost -Port $LocalOverlayPort -WarningAction SilentlyContinue
    if (-not $tcp.TcpTestSucceeded) {
        Fail "Local overlay listener is not reachable at $LocalOverlayHost`:$LocalOverlayPort"
    }
    Write-RunLog "Local overlay listener seems reachable"
}

function Get-RemoteArtifactForSite {
    param(
        [string]$Site,
        [string]$SinceWindow
    )

    # One-line command to avoid quoting/newline issues when passing to plink.
    # grep -F: treat patterns as literals.
    $exitPattern = "exit=Udp://$($UsaHost):$($UsaPort)"
    $remoteCmd = "journalctl -u $ExitUnit --since '$SinceWindow' --no-pager | " +
        "grep 'VALIDATION_ARTIFACT_FINAL' | " +
        "grep -F 'is_final=1' | " +
        "grep -F '$exitPattern' | " +
        "grep -F 'site=$Site' | " +
        "tail -n 1"

    try {
        # Pass remoteCmd as a single quoted argument to avoid tokenization issues.
        $output = & $plinkPath -batch -ssh -pw $UsaSshPassword "$UsaSshUser@$UsaHost" "$remoteCmd" 2>$null
        if ($null -eq $output) {
            Write-RunLog "DEBUG: empty plink output for site=$Site since='$SinceWindow' remoteCmd=$remoteCmd"
            return ""
        }

        $outputStr = ($output | Out-String).Trim()
        if ([string]::IsNullOrWhiteSpace($outputStr)) {
            Write-RunLog "DEBUG: no matching artifact for site=$Site since='$SinceWindow' remoteCmd=$remoteCmd"
            return ""
        }

        return $outputStr
    } catch {
        return ""
    }
}

function Test-ArtifactExit {
    param([string]$Artifact)
    if ([string]::IsNullOrWhiteSpace($Artifact)) { return $false }
    return $Artifact -match [regex]::Escape("exit=Udp://$($UsaHost):$($UsaPort)")
}

function Run-DirectOnce {
    param(
        [string]$Site,
        [int]$RunIndex
    )

    Write-RunLog "DIRECT run $RunIndex/$RepeatsPerSite for $Site"

    $curlOutput = ""
    try {
        $curlOutput = & curl.exe -sk -o NUL `
            --max-time $CurlTimeout `
            -w "connect=%{time_connect} appconnect=%{time_appconnect} starttransfer=%{time_starttransfer} total=%{time_total} code=%{http_code}`n" `
            "https://$Site/" 2>$null
    } catch {
        $curlOutput = ""
    }

    $m = Parse-CurlMetrics $curlOutput

    Write-RunLog ("DIRECT {0} run={1} connect={2} appconnect={3} starttransfer={4} total={5} code={6}" -f `
        $Site, $RunIndex, $m.connect, $m.appconnect, $m.starttransfer, $m.total, $m.code)

    Append-CsvRow `
        -Site $Site `
        -Mode "direct" `
        -Run $RunIndex `
        -ExitField "" `
        -Route "" `
        -Connect $m.connect `
        -AppConnect $m.appconnect `
        -StartTransfer $m.starttransfer `
        -Total $m.total `
        -Code $m.code `
        -RespBytes "" `
        -FramesSent "" `
        -AvgInflight "" `
        -MaxInflight "" `
        -RetransmitRate "" `
        -ThroughputBps "" `
        -AckLatencyMsAvg "" `
        -ArtifactRaw ""
}

function Run-OverlayOnce {
    param(
        [string]$Site,
        [int]$RunIndex
    )

    Write-RunLog "OVERLAY run $RunIndex/$RepeatsPerSite for $Site"

    $curlOutput = ""
    try {
        $curlOutput = & curl.exe -sk -o NUL `
            --max-time $CurlTimeout `
            --connect-to "$Site`:443:$LocalOverlayHost`:$LocalOverlayPort" `
            -w "connect=%{time_connect} appconnect=%{time_appconnect} starttransfer=%{time_starttransfer} total=%{time_total} code=%{http_code}`n" `
            "https://$Site/" 2>$null
    } catch {
        $curlOutput = ""
    }

    Start-Sleep -Milliseconds $LogWaitMs

    # Query a narrow window (better matching the overlay request just executed).
    $since = $RemoteSinceWindow
    $artifact = Get-RemoteArtifactForSite -Site $Site -SinceWindow $since
    Add-Content -Path $ArtifactLog -Value $artifact -Encoding UTF8

    $m = Parse-CurlMetrics $curlOutput

    $exitField       = Parse-ArtifactField $artifact "exit"
    $route           = Parse-ArtifactField $artifact "route"
    $respBytes       = Parse-ArtifactField $artifact "resp_bytes"
    $framesSent      = Parse-ArtifactField $artifact "frames_sent"
    $avgInflight     = Parse-ArtifactField $artifact "avg_inflight"
    $maxInflight     = Parse-ArtifactField $artifact "max_inflight"
    $retransmitRate  = Parse-ArtifactField $artifact "retransmit_rate"
    $throughputBps   = Parse-ArtifactField $artifact "throughput_bps"
    $ackLatencyMsAvg = Parse-ArtifactField $artifact "ack_latency_ms_avg"

    if (Test-ArtifactExit $artifact) {
        Write-RunLog ("OVERLAY {0} run={1} OK exit={2} route={3} total={4} code={5} avg_inflight={6} max_inflight={7}" -f `
            $Site, $RunIndex, $exitField, $route, $m.total, $m.code, $avgInflight, $maxInflight)
    } else {
        Write-RunLog "OVERLAY $Site run=$RunIndex WARNING: no valid USA artifact matched (expected exit=Udp://$($UsaHost):$($UsaPort))"
    }

    Append-CsvRow `
        -Site $Site `
        -Mode "overlay" `
        -Run $RunIndex `
        -ExitField $exitField `
        -Route $route `
        -Connect $m.connect `
        -AppConnect $m.appconnect `
        -StartTransfer $m.starttransfer `
        -Total $m.total `
        -Code $m.code `
        -RespBytes $respBytes `
        -FramesSent $framesSent `
        -AvgInflight $avgInflight `
        -MaxInflight $maxInflight `
        -RetransmitRate $retransmitRate `
        -ThroughputBps $throughputBps `
        -AckLatencyMsAvg $ackLatencyMsAvg `
        -ArtifactRaw $artifact
}

function Print-SummaryHint {
    Write-Host "Done."
    Write-Host "Generated files:"
    Write-Host "  - $ResultsCsv"
    Write-Host "  - $ArtifactLog"
    Write-Host "  - $RunLog"
    Write-Host ""
    Write-Host "Next step:"
    Write-Host "  cargo run --bin validate_report -- --input $ResultsCsv --output summary_usa.csv"
    Write-Host ""
    Write-Host "Quick checks:"
    Write-Host "  1) overlay rows should contain exit=Udp://$UsaHost`:$UsaPort"
    Write-Host "  2) overlay rows should NOT contain exit=Udp://127.0.0.1"
    Write-Host "  3) for heavy sites: avg_inflight/max_inflight/retransmit_rate/throughput_bps are the key transport metrics"
}

# Main
Require-File -Path $plinkPath
if (-not (Get-Command "curl.exe" -ErrorAction SilentlyContinue)) {
    Fail "Missing required command: curl.exe (must be in PATH)"
}
if ([string]::IsNullOrWhiteSpace($UsaSshPassword)) {
    Fail "Set USA_SSH_PASS environment variable (or edit UsaSshPassword param) before running."
}

Init-Files

Write-RunLog "Starting USA validation"
Write-RunLog "Sites: $($Sites -join ', ')"
Write-RunLog "Repeats per site: $RepeatsPerSite"
Write-RunLog "Expected exit: Udp://$UsaHost`:$UsaPort"

Test-LocalOverlayListener

for ($si = 0; $si -lt $Sites.Count; $si++) {
    $site = $Sites[$si]
    for ($i = 1; $i -le $RepeatsPerSite; $i++) {
        Run-DirectOnce -Site $site -RunIndex $i
        Run-OverlayOnce -Site $site -RunIndex $i
    }
}

Write-RunLog "Validation completed successfully"
Print-SummaryHint

