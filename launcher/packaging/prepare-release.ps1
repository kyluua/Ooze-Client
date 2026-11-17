[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OozeJar,
    [Parameter(Mandatory)][string]$CliExe,
    [Parameter(Mandatory)][string]$GuiExe,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$PinnedPackageDirectory
)
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) { throw 'Use a new empty output path. Existing packages are never overwritten.' }
foreach ($inputPath in @($OozeJar, $CliExe, $GuiExe)) { if (-not (Test-Path -LiteralPath $inputPath -PathType Leaf)) { throw "Missing build input: $inputPath" } }
New-Item -ItemType Directory -Path $outputRoot | Out-Null
$metadataRoot = Join-Path $outputRoot 'metadata'
$scratchRoot = Join-Path $outputRoot '.scratch'
New-Item -ItemType Directory -Path $metadataRoot, $scratchRoot | Out-Null
function Canonical-Value($Value) {
    if ($Value -is [Collections.IDictionary]) {
        $ordered=[ordered]@{}
        [string[]]$keys=@($Value.Keys)
        [Array]::Sort($keys,[StringComparer]::Ordinal)
        foreach($key in $keys){ $ordered[$key]=Canonical-Value $Value[$key] }
        return $ordered
    }
    if ($Value -is [Collections.IEnumerable] -and $Value -isnot [string]) {
        $items=@(foreach($item in $Value){ Canonical-Value $item })
        return ,$items
    }
    return $Value
}
function Write-Json($Path, $Value) { [IO.File]::WriteAllText($Path, (ConvertTo-Json -InputObject (Canonical-Value $Value) -Depth 30), [Text.UTF8Encoding]::new($false)) }
function Local-Artifact($FilePath, $Relative) {
    @{file=$Relative;url=$null;size=(Get-Item -LiteralPath $FilePath).Length;sha256=(Get-FileHash -LiteralPath $FilePath -Algorithm SHA256).Hash.ToLowerInvariant();sha1=$null}
}
function Remote-Artifact($Url, $Name) {
    if (-not $Url.StartsWith('https://maven.fabricmc.net/', [StringComparison]::Ordinal)) { throw 'Unexpected Maven URL' }
    $downloadPath = Join-Path $scratchRoot $Name
    Invoke-WebRequest -Uri $Url -OutFile $downloadPath -TimeoutSec 120
    $artifact = Local-Artifact $downloadPath $null
    $artifact.file=$null; $artifact.url=$Url
    return $artifact
}
if ($PinnedPackageDirectory) {
    $release = Get-Content -LiteralPath (Join-Path $PinnedPackageDirectory 'release.json') -Raw | ConvertFrom-Json -AsHashtable
    foreach ($name in @('minecraft.json','fabric.json','sources.json')) { Copy-Item -LiteralPath (Join-Path $PinnedPackageDirectory "metadata\$name") -Destination (Join-Path $metadataRoot $name) }
} else {
    $versionManifestUrl = 'https://piston-meta.mojang.com/mc/game/version_manifest_v2.json'
    $versions = Invoke-RestMethod $versionManifestUrl -TimeoutSec 30
    $minecraftVersion = @($versions.versions | Where-Object id -eq '1.21.11')[0]
    if (-not $minecraftVersion) { throw 'Required Minecraft version unavailable' }
    $minecraftPath = Join-Path $metadataRoot 'minecraft.json'
    Invoke-WebRequest $minecraftVersion.url -OutFile $minecraftPath -TimeoutSec 30
    if ((Get-FileHash -LiteralPath $minecraftPath -Algorithm SHA1).Hash -ine $minecraftVersion.sha1) { throw 'Minecraft metadata hash mismatch' }
    $fabricUrl = 'https://meta.fabricmc.net/v2/versions/loader/1.21.11/0.20.1/profile/json'
    $fabricPath = Join-Path $metadataRoot 'fabric.json'
    Invoke-WebRequest $fabricUrl -OutFile $fabricPath -TimeoutSec 30
    $fabric = Get-Content -LiteralPath $fabricPath -Raw | ConvertFrom-Json
    $libraries = @()
    foreach ($library in $fabric.libraries) {
        $parts = $library.name.Split(':')
        if ($parts.Length -ne 3 -or @($parts | Where-Object { $_ -notmatch '^[a-zA-Z0-9._+\-]+$' }).Count) { throw 'Unexpected Fabric coordinate' }
        $path = $parts[0].Replace('.','/')+'/'+$parts[1]+'/'+$parts[2]+'/'+$parts[1]+'-'+$parts[2]+'.jar'
        $artifact = Remote-Artifact ('https://maven.fabricmc.net/'+$path) ($parts[1]+'.jar')
        if ($library.sha256 -and $library.sha256 -ine $artifact.sha256) { throw 'Fabric library checksum mismatch' }
        $libraries += @{name=$library.name;path=$path;artifact=$artifact}
    }
    $apiUrl = 'https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/0.141.6+1.21.11/fabric-api-0.141.6+1.21.11.jar'
    $fabricApi = Remote-Artifact $apiUrl 'fabric-api.jar'
    $runtimeUrl = 'https://api.adoptium.net/v3/assets/latest/21/hotspot?architecture=x64&image_type=jre&os=windows&vendor=eclipse'
    $runtimes = Invoke-RestMethod $runtimeUrl -TimeoutSec 30
    $runtime = $runtimes[0]
    $package = $runtime.binary.package
    if (-not $package.link.StartsWith('https://github.com/adoptium/temurin21-binaries/releases/download/')) { throw 'Unexpected Java vendor download' }
    $javaFolder = $runtime.version.openjdk_version.Split('+')[0]
    $javaRelative = 'jdk-'+$javaFolder+'+'+$runtime.version.build+'-jre/bin/java.exe'
    $release = @{
        schema=1; id='ooze-l0-mc1.21.11'; minecraft='1.21.11';loader='0.20.1'
        minecraft_metadata=(Local-Artifact $minecraftPath 'metadata/minecraft.json')
        fabric_profile=(Local-Artifact $fabricPath 'metadata/fabric.json')
        fabric_libraries=$libraries;fabric_api=$fabricApi
        java=@{archive=@{url=$package.link;file=$null;size=$package.size;sha256=$package.checksum;sha1=$null};java_relative=$javaRelative}
    }
    Write-Json (Join-Path $metadataRoot 'sources.json') @{
        frozen_utc=[DateTime]::UtcNow.ToString('o');minecraft_manifest=$versionManifestUrl;minecraft_metadata=$minecraftVersion.url
        minecraft_sha1=$minecraftVersion.sha1;fabric_profile=$fabricUrl;fabric_api=$apiUrl;java_metadata=$runtimeUrl;java_license='https://adoptium.net/about/licensing/'
    }
}
Copy-Item -LiteralPath $OozeJar -Destination (Join-Path $outputRoot 'ooze-client.jar')
Copy-Item -LiteralPath $CliExe -Destination (Join-Path $outputRoot 'ooze-launcher-cli.exe')
Copy-Item -LiteralPath $GuiExe -Destination (Join-Path $outputRoot 'ooze-launcher.exe')
$release.ooze = Local-Artifact (Join-Path $outputRoot 'ooze-client.jar') 'ooze-client.jar'
Write-Json (Join-Path $outputRoot 'release.json') $release
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $outputRoot 'LICENSE')
$licenseRoot = Join-Path $outputRoot 'licenses'
New-Item -ItemType Directory -Path $licenseRoot | Out-Null
$cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargoExe)) { $cargoExe = (Get-Command cargo -ErrorAction Stop).Source }
$cargoMetadata = & $cargoExe metadata --manifest-path (Join-Path $repoRoot 'launcher\Cargo.toml') --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE) { throw 'Cargo metadata failed' }
$notices = @()
foreach ($dependency in @($cargoMetadata.packages | Where-Object source)) {
    $notices += @{name=$dependency.name;version=$dependency.version;license=$dependency.license;repository=$dependency.repository}
    $dependencyRoot = Split-Path -Parent $dependency.manifest_path
    foreach ($license in @(Get-ChildItem -LiteralPath $dependencyRoot -File | Where-Object Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)')) {
        Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $licenseRoot ($dependency.name+'-'+$dependency.version+'-'+$license.Name))
    }
}
Write-Json (Join-Path $licenseRoot 'dependencies.json') $notices
foreach ($license in @(Get-ChildItem -LiteralPath (Join-Path $repoRoot 'native\licenses') -File)) { Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $licenseRoot ('rust-'+$license.Name)) }
$fontsRoot = Join-Path $repoRoot 'launcher\resources\fonts'
Copy-Item -LiteralPath $fontsRoot -Destination (Join-Path $licenseRoot 'fonts') -Recurse
@'
Ooze Launcher review package — Windows x64. Start ooze-launcher.exe for the desktop interface.
Microsoft sign-in requires Ooze's owned public OAuth client ID and valid Minecraft Services access.
Set OOZE_MICROSOFT_CLIENT_ID in developer builds; players should receive a configured final build.
Run: ooze-launcher-cli.exe status
Then: ooze-launcher-cli.exe install
Then: ooze-launcher-cli.exe sign-in
Then: ooze-launcher-cli.exe play
Optional: ooze-launcher-cli.exe settings --java "C:\path\to\Java21\bin\java.exe"
Default storage: %LOCALAPPDATA%\OozeLauncher. Minecraft files and Java download at install.
The CLI is also included for diagnostics. This is an unsigned review build.
This package proves no live sign-in or title-screen launch until acceptance evidence says so.
'@ | Set-Content -LiteralPath (Join-Path $outputRoot 'README.txt') -Encoding utf8NoBOM
# Remove only the scratch directory created above, after verifying its resolved absolute confinement.
$scratchFull = [IO.Path]::GetFullPath($scratchRoot)
if (-not $scratchFull.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Scratch path escaped package' }
Remove-Item -LiteralPath $scratchFull -Recurse -Force
$hashLines = @(Get-ChildItem -LiteralPath $outputRoot -File -Recurse | Sort-Object FullName | ForEach-Object {
    (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()+'  '+[IO.Path]::GetRelativePath($outputRoot,$_.FullName).Replace('\','/')
})
[IO.File]::WriteAllLines((Join-Path $outputRoot 'SHA256SUMS'),$hashLines,[Text.UTF8Encoding]::new($false))
Write-Output "Prepared pinned release at $outputRoot"
