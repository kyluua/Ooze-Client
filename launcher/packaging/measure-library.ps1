[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$CliExe,
    [Parameter(Mandatory)][string]$DataDirectory,
    [Parameter(Mandatory)][string]$InstanceId,
    [Parameter(Mandatory)][string]$OutputJson,
    [ValidateRange(1,10)][int]$Samples=3
)
$ErrorActionPreference='Stop'
if($InstanceId -notmatch '^[a-f0-9]{32}$'){throw 'Invalid instance ID'}
$cli=(Resolve-Path -LiteralPath $CliExe).Path
if(Test-Path -LiteralPath $OutputJson){throw 'Use a fresh benchmark output file'}
$measurements=@()
foreach($scenario in @('startup-list','startup-verify')){
    for($sample=0;$sample -lt $Samples;$sample++){
        $info=[Diagnostics.ProcessStartInfo]::new()
        $info.FileName=$cli
        $info.UseShellExecute=$false
        $info.CreateNoWindow=$true
        $info.RedirectStandardOutput=$true
        $info.RedirectStandardError=$true
        foreach($argument in @('--data-dir',$DataDirectory,'library')){$info.ArgumentList.Add($argument)}
        if($scenario -eq 'startup-list'){$info.ArgumentList.Add('list')}else{$info.ArgumentList.Add('verify');$info.ArgumentList.Add($InstanceId)}
        $process=[Diagnostics.Process]::new();$process.StartInfo=$info
        $timer=[Diagnostics.Stopwatch]::StartNew()
        if(-not $process.Start()){throw 'Could not start benchmark CLI'}
        $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
        $peak=0L
        while(-not $process.HasExited){
            try{$process.Refresh();$peak=[Math]::Max($peak,$process.PeakWorkingSet64)}catch{}
            if($timer.Elapsed.TotalSeconds -gt 30){$process.Kill();throw 'Benchmark CLI timed out'}
            Start-Sleep -Milliseconds 5
        }
        $process.WaitForExit();$timer.Stop()
        $text=$stdout.GetAwaiter().GetResult();$null=$stderr.GetAwaiter().GetResult()
        if($process.ExitCode -ne 0){throw 'Benchmark operation failed; inspect the CLI separately'}
        $null=$text|ConvertFrom-Json
        $measurements+=@{scenario=$scenario;sample=$sample;elapsed_ms=[Math]::Round($timer.Elapsed.TotalMilliseconds,3);observed_peak_working_set_bytes=if($peak -gt 0){$peak}else{$null}}
        $process.Dispose()
    }
}
$report=@{
    schema=1;cli_sha256=(Get-FileHash -LiteralPath $cli).Hash.ToLowerInvariant()
    os=[Runtime.InteropServices.RuntimeInformation]::OSDescription
    architecture=[Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
    samples=$measurements
    conditions='Fresh CLI process each sample; existing isolated test library; OS file cache not flushed; process working set sampled at 5 ms, not allocator heap accounting.'
    unmeasured=@('GUI frame time','JNI/copy/serialization cost','Java allocation','native allocator retained/peak bytes','GPU residency','Java baseline comparison')
}
$parent=Split-Path -Parent ([IO.Path]::GetFullPath($OutputJson))
if(-not(Test-Path -LiteralPath $parent)){New-Item -ItemType Directory -Path $parent|Out-Null}
$report|ConvertTo-Json -Depth 6|Set-Content -LiteralPath $OutputJson -Encoding utf8NoBOM
$report.samples|ConvertTo-Json -Depth 4 -Compress
