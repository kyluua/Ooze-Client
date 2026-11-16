[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$JavaHome,
    [Parameter(Mandatory)][string]$NativeResources,
    [Parameter(Mandatory)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$outRoot=[IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outRoot) { throw 'Choose a new evidence output directory.' }
$javaExe=Join-Path $JavaHome 'bin\java.exe'
$version=& $javaExe -version 2>&1
if ($LASTEXITCODE -or "$version" -notmatch 'version "21\.') { throw 'An actual Java 21 JDK is required.' }
$env:JAVA_HOME=$JavaHome
New-Item -ItemType Directory -Path $outRoot | Out-Null
$cases=@(
    @{name='dev-default';task='runClientGameTest';resources=$NativeResources;state='LOADED';fixture=$true},
    @{name='prod-default';task='runProductionClientGameTest';resources=$NativeResources;state='LOADED';fixture=$false}
)
foreach ($variant in @('missing','corrupt','abi')) {
    $path=Join-Path $outRoot $variant
    New-Item -ItemType Directory -Path $path | Out-Null
    Get-ChildItem -LiteralPath $NativeResources -Force | Copy-Item -Destination $path -Recurse
    $dll=Join-Path $path 'natives\windows-x86_64\ooze_jni.dll'
    if ($variant -eq 'missing') { Remove-Item -LiteralPath $dll }
    if ($variant -eq 'corrupt') { $bytes=[IO.File]::ReadAllBytes($dll);$bytes[4096]=$bytes[4096] -bxor 1;[IO.File]::WriteAllBytes($dll,$bytes) }
    if ($variant -eq 'abi') { $properties=Join-Path $path 'META-INF\ooze\native.properties';$text=[IO.File]::ReadAllText($properties) -replace '(?m)^bridge.abi=.*','bridge.abi=2';[IO.File]::WriteAllText($properties,$text,[Text.UTF8Encoding]::new($false)) }
    $state=if ($variant -eq 'missing') {'MISSING_LIBRARY'} else {'INCOMPATIBLE'}
    $cases+=@{name="win-real-$variant";task='runClientGameTest';resources=$path;state=$state;fixture=$false}
}
$summary=@("Windows x64; Java: $version",'Development/production gametests use Fabric test credentials; this is not launcher authentication evidence.')
$failed=0
Push-Location $repoRoot
try {
    foreach ($case in $cases) {
        $log=Join-Path $outRoot ($case.name+'.log')
        $arguments=@(":versions:mc1_21_11:$($case.task)","-PoozeNativeResources=$($case.resources)","-PoozeExpectEngineState=$($case.state)",'--no-daemon','--console=plain')
        if ($case.name -eq 'win-real-missing') { $arguments+=@('-x',':bridge-java:verifyNativeArtifact') }
        $caseStart=[DateTime]::UtcNow
        & .\gradlew.bat @arguments *> $log
        $code=$LASTEXITCODE
        $text=[IO.File]::ReadAllText($log)
        $passed=$code -eq 0 -and $text.Contains("C0 launch test PASSED: state=$($case.state)")
        if ($case.fixture) { $passed=$passed -and $text.Contains('C0 bridge probe ok=true') -and $text.Contains('C0 bridge fixture PASSED') }
        $result=if ($passed) {'PASS'} else {'FAIL'}
        if (-not $passed) { $failed++ }
        $summary+="$($case.name): $result; expected $($case.state); exit $code"
        Write-Output $summary[-1]
        $excerpts=@($text -split '\r?\n' | Where-Object { $_ -match 'native engine loaded|state    :|bootstrap:|C0 bridge|C0 launch test PASSED|AssertionError|BUILD FAILED|BUILD SUCCESSFUL' })
        [IO.File]::WriteAllLines((Join-Path $outRoot ($case.name+'.excerpt.txt')),$excerpts,[Text.UTF8Encoding]::new($false))
        $shotDir=if ($case.task -eq 'runClientGameTest') {'clientGameTest'} else {'productionClientGameTest'}
        $shots=Join-Path $repoRoot "versions\mc1_21_11\build\run\$shotDir\screenshots"
        if (Test-Path -LiteralPath $shots) {Get-ChildItem -LiteralPath $shots -Filter '*.png' | Where-Object LastWriteTimeUtc -ge $caseStart | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $outRoot ($case.name+'--'+$_.Name))}}
    }
} finally {Pop-Location}
[IO.File]::WriteAllLines((Join-Path $outRoot 'SUMMARY.txt'),$summary,[Text.UTF8Encoding]::new($false))
if ($failed) {throw "$failed C0 Windows checks failed. Inspect the saved logs."}
