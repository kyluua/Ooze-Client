[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$JavaHome,
    [switch]$IncludeFaultInjection
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$nativeRoot = Join-Path $repoRoot 'native'
$bridgeRoot = Join-Path $repoRoot 'bridge-java'
$javaExe = Join-Path $JavaHome 'bin\java.exe'
$javacExe = Join-Path $JavaHome 'bin\javac.exe'
$jarExe = Join-Path $JavaHome 'bin\jar.exe'
foreach ($tool in @($javaExe, $javacExe, $jarExe)) {
    if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) { throw "Missing JDK tool: $tool" }
}
$compilerVersion = & $javacExe -version 2>&1
if ($LASTEXITCODE -ne 0 -or "$compilerVersion" -notmatch '^javac 21[. ]') {
    throw "Use an actual Java 21 JDK. Found: $compilerVersion"
}
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoExe = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $cargoExe -PathType Leaf)) { throw 'Install rustup with the Windows MSVC toolchain.' }
$env:Path = (Split-Path -Parent $cargoExe) + [IO.Path]::PathSeparator + $env:Path

function Invoke-Cargo {
    param([string[]]$CargoArguments)
    & $cargoExe @CargoArguments
    if ($LASTEXITCODE -ne 0) { throw "cargo failed: $CargoArguments" }
}

function Write-Utf8 {
    param([string]$Path, [string]$Content)
    [IO.File]::WriteAllText($Path, $Content, [Text.UTF8Encoding]::new($false))
}

Push-Location $repoRoot
try {
    Invoke-Cargo @('fmt', '--manifest-path', 'native/Cargo.toml', '--all', '--', '--check')
    Invoke-Cargo @('clippy', '--manifest-path', 'native/Cargo.toml', '--locked', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-Cargo @('test', '--manifest-path', 'native/Cargo.toml', '--locked', '--workspace', '--all-features')

    $metadataText = & $cargoExe metadata --manifest-path native/Cargo.toml --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata = $metadataText | ConvertFrom-Json
    $sourceCommit = (& git rev-parse HEAD).Trim()
    $sourceDirty = if (@(& git status --porcelain).Count -eq 0) { 'false' } else { 'true' }
    $variants = @('release')
    if ($IncludeFaultInjection) { $variants += 'fault-injection' }

    foreach ($variant in $variants) {
        $targetDirectory = Join-Path $nativeRoot "target\g0-$variant"
        $cargoBuild = @('build', '--manifest-path', 'native/Cargo.toml', '--locked', '--release', '--target', 'x86_64-pc-windows-msvc', '--target-dir', $targetDirectory, '-p', 'ooze-jni')
        if ($variant -eq 'fault-injection') { $cargoBuild += @('--features', 'fault-injection') }
        Invoke-Cargo $cargoBuild
        $library = Join-Path $targetDirectory 'x86_64-pc-windows-msvc\release\ooze_jni.dll'
        $artifactRoot = Join-Path $nativeRoot "dist\g0\$variant"
        $resourcesRoot = Join-Path $artifactRoot 'resources'
        $libraryDirectory = Join-Path $resourcesRoot 'natives\windows-x86_64'
        $metadataDirectory = Join-Path $resourcesRoot 'META-INF\ooze'
        $mainClasses = Join-Path $bridgeRoot "build\g0\$variant\main"
        $testClasses = Join-Path $bridgeRoot "build\g0\$variant\test"
        foreach ($directory in @($libraryDirectory, $metadataDirectory, $mainClasses, $testClasses)) {
            New-Item -ItemType Directory -Force -Path $directory | Out-Null
        }
        Copy-Item -LiteralPath $library -Destination (Join-Path $libraryDirectory 'ooze_jni.dll') -Force
        $hash = (Get-FileHash -LiteralPath $library -Algorithm SHA256).Hash.ToLowerInvariant()
        $capabilities = if ($variant -eq 'fault-injection') { 7 } else { 3 }
        $manifest = @"
schema=1
library.version=0.1.0-g0
bridge.abi=1
platform=windows-x86_64
rust.target=x86_64-pc-windows-msvc
resource=natives/windows-x86_64/ooze_jni.dll
sha256=$hash
capabilities=$capabilities
variant=$variant
source.commit=$sourceCommit
source.dirty=$sourceDirty
"@
        Write-Utf8 (Join-Path $metadataDirectory 'native.properties') $manifest
        Copy-Item -LiteralPath (Join-Path $repoRoot 'docs\bridge\resources.json') -Destination (Join-Path $metadataDirectory 'resources.json') -Force
        Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $metadataDirectory 'LICENSE') -Force

        $notices = @('Ooze G0 Rust dependency notices. Each dependency retains its upstream license.', '')
        foreach ($package in @($metadata.packages | Where-Object { $_.source })) {
            $notices += "$($package.name) $($package.version): $($package.license)"
            $packageRoot = Split-Path -Parent $package.manifest_path
            $licenseFiles = @(Get-ChildItem -LiteralPath $packageRoot -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' })
            if ($package.license_file) {
                $extraLicense = Join-Path $packageRoot $package.license_file
                if (Test-Path -LiteralPath $extraLicense -PathType Leaf) { $licenseFiles += Get-Item -LiteralPath $extraLicense }
            }
            $noticeDirectory = Join-Path $metadataDirectory "licenses\$($package.name)-$($package.version)"
            New-Item -ItemType Directory -Force -Path $noticeDirectory | Out-Null
            foreach ($licenseFile in $licenseFiles) {
                Copy-Item -LiteralPath $licenseFile.FullName -Destination (Join-Path $noticeDirectory $licenseFile.Name) -Force
            }
        }
        Write-Utf8 (Join-Path $metadataDirectory 'THIRD_PARTY.txt') ($notices -join "`n")
        foreach ($rustLicense in @(Get-ChildItem -LiteralPath (Join-Path $nativeRoot 'licenses') -Filter 'RUST-*' -File)) {
            Copy-Item -LiteralPath $rustLicense.FullName -Destination (Join-Path $metadataDirectory $rustLicense.Name) -Force
        }
        $rustSysroot = (& rustc --print sysroot).Trim()
        $standardLibraryNotices = Join-Path $rustSysroot 'share\doc\rust\COPYRIGHT-library.html'
        if (-not (Test-Path -LiteralPath $standardLibraryNotices -PathType Leaf)) {
            throw 'Pinned Rust toolchain is missing COPYRIGHT-library.html; cannot package standard-library notices.'
        }
        Copy-Item -LiteralPath $standardLibraryNotices -Destination (Join-Path $metadataDirectory 'RUST-COPYRIGHT-library.html') -Force
        Copy-Item -LiteralPath (Join-Path $metadataDirectory 'native.properties') -Destination (Join-Path $artifactRoot 'native-artifact.properties') -Force

        $mainSources = @(Get-ChildItem -LiteralPath (Join-Path $bridgeRoot 'src\main\java') -Filter '*.java' -Recurse | ForEach-Object FullName)
        & $javacExe --release 21 -Xlint:all -Werror -d $mainClasses @mainSources
        if ($LASTEXITCODE -ne 0) { throw 'Java bridge compilation failed' }
        $testSources = @(Get-ChildItem -LiteralPath (Join-Path $bridgeRoot 'src\test\java') -Filter '*.java' -Recurse | ForEach-Object FullName)
        & $javacExe --release 21 -Xlint:all -Werror -classpath $mainClasses -d $testClasses @testSources
        if ($LASTEXITCODE -ne 0) { throw 'Java harness compilation failed' }
        $jarFile = Join-Path $artifactRoot "ooze-bridge-java-0.1.0-g0-windows-x86_64-$variant.jar"
        & $jarExe --create --file $jarFile -C $mainClasses . -C $resourcesRoot .
        if ($LASTEXITCODE -ne 0) { throw 'Bridge packaging failed' }
        $classPath = $jarFile + [IO.Path]::PathSeparator + $testClasses
        $harnessArguments = @('-Xcheck:jni', '-Djava.awt.headless=true', '-classpath', $classPath, 'io.github.kyluua.ooze.bridge.BridgeHarness')
        if ($variant -eq 'fault-injection') {
            $harnessArguments = @('-Dooze.bridge.allowFaultInjection=true') + $harnessArguments + '--fault-injection'
        }
        & $javaExe @harnessArguments
        if ($LASTEXITCODE -ne 0) { throw "Real Java 21/JNI harness failed ($variant)" }
        $jarHash = (Get-FileHash -LiteralPath $jarFile -Algorithm SHA256).Hash.ToLowerInvariant()
        Write-Utf8 (Join-Path $artifactRoot 'SHA256SUMS') "$hash  resources/natives/windows-x86_64/ooze_jni.dll`n$jarHash  $([IO.Path]::GetFileName($jarFile))`n"
        Write-Output "G0 artifact ($variant): $jarFile"
        Write-Output "Native SHA-256: $hash"
    }
} finally {
    Pop-Location
}
