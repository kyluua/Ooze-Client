[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BasePackageDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$ValidatedClientManifest
)

$ErrorActionPreference = 'Stop'
$base = (Resolve-Path -LiteralPath $BasePackageDirectory).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw 'Choose a new catalog package directory.' }
$legacy = Get-Content -LiteralPath (Join-Path $base 'release.json') -Raw | ConvertFrom-Json -AsHashtable
if ($legacy.schema -ne 1 -or $legacy.minecraft -ne '1.21.11' -or $legacy.loader -ne '0.20.1') {
    throw 'Expected the existing 1.21.11 base package.'
}

$versions = @('1.21.11','26.1','26.1.1','26.1.2','26.2','26.3')
$clients = @{}
if ($ValidatedClientManifest) {
    $clientPath = (Resolve-Path -LiteralPath $ValidatedClientManifest).Path
    $clientRoot = Split-Path -Parent $clientPath
    $clientIndex = Get-Content -LiteralPath $clientPath -Raw | ConvertFrom-Json
    if ($clientIndex.schema -ne 1) { throw 'Unsupported validated client manifest schema.' }
    foreach ($client in $clientIndex.targets) {
        if ($client.minecraft -eq '1.21.11' -or $client.minecraft -notin $versions -or
            $clients.ContainsKey($client.minecraft) -or $client.java_major -ne 25 -or $client.loader -ne $legacy.loader -or
            $client.file -notmatch '^[A-Za-z0-9._+-]+\.jar$' -or $client.sha256 -notmatch '^[a-f0-9]{64}$' -or
            $client.api_version -notmatch '^[A-Za-z0-9._+-]+$' -or [string]::IsNullOrWhiteSpace($client.evidence)) {
            throw 'Invalid or unverified client manifest entry.'
        }
        $source = Join-Path $clientRoot $client.file
        if ((Get-FileHash -LiteralPath $source).Hash -ine $client.sha256) {
            throw "Validated Ooze JAR changed for $($client.minecraft)."
        }
        $clients[$client.minecraft] = $client
    }
}

function Get-LocalPin([string]$Relative) {
    $item = Get-Item -LiteralPath (Join-Path $output $Relative)
    [ordered]@{ file=$Relative; url=$null; size=$item.Length; sha256=(Get-FileHash -LiteralPath $item.FullName).Hash.ToLowerInvariant(); sha1=$null }
}
function Get-MavenPin([string]$Relative, $Published) {
    $url = 'https://maven.fabricmc.net/' + $Relative
    if (!$mavenPins.ContainsKey($url)) {
        Invoke-WebRequest -Uri $url -OutFile $scratch -TimeoutSec 90
        $item = Get-Item -LiteralPath $scratch
        $mavenPins[$url] = [ordered]@{
            file=$null; url=$url; size=$item.Length
            sha256=(Get-FileHash -LiteralPath $scratch -Algorithm SHA256).Hash.ToLowerInvariant()
            sha1=(Get-FileHash -LiteralPath $scratch -Algorithm SHA1).Hash.ToLowerInvariant()
        }
    }
    $pin = $mavenPins[$url]
    if (($Published.sha256 -and $Published.sha256 -ine $pin.sha256) -or
        ($Published.sha1 -and $Published.sha1 -ine $pin.sha1) -or
        ($Published.size -and $Published.size -ne $pin.size)) {
        throw "Fabric Maven download does not match Fabric Meta: $Relative"
    }
    $pin
}

$major25 = @(Invoke-RestMethod 'https://api.adoptium.net/v3/assets/latest/25/hotspot?architecture=x64&image_type=jre&os=windows&vendor=eclipse' -TimeoutSec 30)[0]
$runtime = $major25.binary.package
if ($major25.version.major -ne 25 -or $runtime.checksum -notmatch '^[a-f0-9]{64}$' -or $runtime.size -le 0 -or
    $runtime.link -notmatch '^https://github\.com/adoptium/temurin25-binaries/releases/download/[^/]+/OpenJDK25U-jre_x64_windows_hotspot_[A-Za-z0-9._]+\.zip$') {
    throw 'Adoptium did not return the expected Windows x64 Java 25 archive.'
}
$java25 = [ordered]@{
    archive=[ordered]@{ file=$null; url=$runtime.link; size=$runtime.size; sha256=$runtime.checksum; sha1=$null }
    java_relative=('jdk-' + ($major25.version.openjdk_version -split '\+')[0] + '+' + $major25.version.build + '-jre/bin/java.exe')
}
$mojangUrl = 'https://piston-meta.mojang.com/mc/game/version_manifest_v2.json'
$manifest = Invoke-RestMethod $mojangUrl -TimeoutSec 30
$mavenPins = @{}
$scratch = [IO.Path]::GetTempFileName()
try {
    Copy-Item -LiteralPath $base -Destination $output -Recurse
    $rows = foreach ($version in $versions) {
        if ($version -eq '1.21.11') {
            $gamePin = $legacy.minecraft_metadata
            $java = $legacy.java
            $major = 21
            $fabric = [ordered]@{
                loader=$legacy.loader; profile=$legacy.fabric_profile; libraries=$legacy.fabric_libraries
                bundle=[ordered]@{api_version='0.141.6+1.21.11'; api=$legacy.fabric_api; ooze=$legacy.ooze}
            }
        } else {
            $entry = @($manifest.versions | Where-Object { $_.id -eq $version -and $_.type -eq 'release' })
            if ($entry.Count -ne 1 -or $entry[0].url -notmatch '^https://piston-meta\.mojang\.com/' -or
                $entry[0].sha1 -notmatch '^[a-f0-9]{40}$') { throw "Missing Mojang release $version." }
            $prefix = "metadata/$version"
            $null = New-Item -ItemType Directory -Path (Join-Path $output $prefix)
            $gameRelative = "$prefix/minecraft.json"
            $game = Join-Path $output $gameRelative
            Invoke-WebRequest -Uri $entry[0].url -OutFile $game -TimeoutSec 30
            if ((Get-FileHash -LiteralPath $game -Algorithm SHA1).Hash -ine $entry[0].sha1) {
                throw "Mojang version metadata hash mismatch for $version."
            }
            $metadata = Get-Content -LiteralPath $game -Raw | ConvertFrom-Json
            if ($metadata.id -ne $version -or $metadata.javaVersion.majorVersion -ne 25) {
                throw "Unexpected Mojang version or Java requirement for $version."
            }
            $gamePin = Get-LocalPin $gameRelative
            $major = 25
            $java = $java25

            $profileRelative = "$prefix/fabric.json"
            $profilePath = Join-Path $output $profileRelative
            Invoke-WebRequest -Uri "https://meta.fabricmc.net/v2/versions/loader/$version/$($legacy.loader)/profile/json" -OutFile $profilePath -TimeoutSec 30
            $profile = Get-Content -LiteralPath $profilePath -Raw | ConvertFrom-Json
            if ($profile.id -ne "fabric-loader-$($legacy.loader)-$version" -or $profile.inheritsFrom -ne $version -or
                $profile.mainClass -ne 'net.fabricmc.loader.impl.launch.knot.KnotClient') {
                throw "Unexpected Fabric profile for $version."
            }
            $libraries = @(foreach ($library in $profile.libraries) {
                $parts = $library.name.Split(':')
                if ($parts.Length -ne 3 -or @($parts | Where-Object { $_ -notmatch '^[A-Za-z0-9._+-]+$' }).Count) {
                    throw 'Invalid Fabric Maven coordinate.'
                }
                $path = "$($parts[0].Replace('.','/'))/$($parts[1])/$($parts[2])/$($parts[1])-$($parts[2]).jar"
                [ordered]@{ name=$library.name; path=$path; artifact=(Get-MavenPin $path $library) }
            })
            $bundle = $null
            if ($clients.ContainsKey($version)) {
                $client = $clients[$version]
                $jarRelative = "$prefix/ooze-client.jar"
                Copy-Item -LiteralPath (Join-Path $clientRoot $client.file) -Destination (Join-Path $output $jarRelative)
                $api = $client.api_version
                $bundle = [ordered]@{
                    api_version=$api
                    api=(Get-MavenPin "net/fabricmc/fabric-api/fabric-api/$api/fabric-api-$api.jar" $null)
                    ooze=(Get-LocalPin $jarRelative)
                }
            }
            $fabric = [ordered]@{ loader=$legacy.loader; profile=(Get-LocalPin $profileRelative); libraries=$libraries; bundle=$bundle }
        }
        [ordered]@{minecraft=$version; java_major=$major; minecraft_metadata=$gamePin; java=$java; fabric=$null}
        [ordered]@{minecraft=$version; java_major=$major; minecraft_metadata=$gamePin; java=$java; fabric=$fabric}
    }
    [ordered]@{schema=2; id='ooze-0.1.0-rc1-windows-version-catalog'; targets=@($rows)} |
        ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $output 'release.json') -Encoding utf8NoBOM
    @'
Ooze Launcher Windows x64 version catalog candidate.
This package is for per-version acceptance testing. See SOL-MULTIVERSION.md for the verification matrix.

Extract together and start ooze-launcher.exe. Use the official browser sign-in when requested.
The candidate has Vanilla and Fabric records for 1.21.11, 26.1, 26.1.1, 26.1.2, 26.2 and 26.3.
The launcher provisions Java 21 for 1.21.11 and Java 25 for 26.x.
Choose a separate instance for each target. Existing instance files stay at their original location.
Ooze client bundles are present only for the validated Fabric records in release.json.
The original 1.21.11 RC1 distribution is available separately.
'@ | Set-Content -LiteralPath (Join-Path $output 'README.txt') -Encoding utf8NoBOM
    $sums = Join-Path $output 'SHA256SUMS'
    $hashes = @(Get-ChildItem -LiteralPath $output -Recurse -File | Where-Object FullName -ne $sums | Sort-Object FullName | ForEach-Object {
        (Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant() + '  ' +
            [IO.Path]::GetRelativePath($output, $_.FullName).Replace('\','/')
    })
    [IO.File]::WriteAllLines($sums, $hashes, [Text.UTF8Encoding]::new($false))
    "Prepared $($rows.Count) pinned version and loader records at $output"
} finally {
    Remove-Item -LiteralPath $scratch
}
