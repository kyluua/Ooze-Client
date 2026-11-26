[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateSet('1.21.11','26.1','26.1.1','26.1.2','26.2','26.3')][string]$MinecraftVersion = '1.21.11'
)

$ErrorActionPreference = 'Stop'
$package = (Resolve-Path -LiteralPath $PackageDirectory).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw 'Choose a new modpack output directory.' }
$release = Get-Content -LiteralPath (Join-Path $package 'release.json') -Raw | ConvertFrom-Json
if ($release.schema -eq 1 -and $MinecraftVersion -eq '1.21.11' -and
    $release.id -eq 'ooze-0.1.0-rc1-mc1.21.11' -and $release.minecraft -eq '1.21.11' -and $release.loader -eq '0.20.1') {
    $loader = $release.loader
    $java = 21
    $bundle = [pscustomobject]@{api_version='0.141.6+1.21.11'; api=$release.fabric_api; ooze=$release.ooze}
} elseif ($release.schema -eq 2) {
    $row = @($release.targets | Where-Object { $_.minecraft -eq $MinecraftVersion -and $_.fabric.bundle })
    if ($row.Count -ne 1) { throw 'No validated Ooze client bundle for this Minecraft version.' }
    $loader = $row[0].fabric.loader
    $java = $row[0].java_major
    $bundle = $row[0].fabric.bundle
} else {
    throw 'The package does not include the selected Ooze client.'
}
$jar = Get-Item -LiteralPath (Join-Path $package $bundle.ooze.file)
function Get-Sha($Path, $Algorithm) {
    (Get-FileHash -LiteralPath $Path -Algorithm $Algorithm).Hash.ToLowerInvariant()
}
if ($jar.Length -ne $bundle.ooze.size -or (Get-Sha $jar.FullName SHA256) -ne $bundle.ooze.sha256) {
    throw 'Ooze JAR differs from the launcher package manifest.'
}
$jarZip = [IO.Compression.ZipFile]::OpenRead($jar.FullName)
try {
    $entry = $jarZip.GetEntry('fabric.mod.json')
    if (!$entry -or !$jarZip.GetEntry('natives/windows-x86_64/ooze_jni.dll') -or
        !$jarZip.GetEntry('META-INF/ooze/native.properties')) { throw 'Missing production mod or Windows engine.' }
    $reader = [IO.StreamReader]::new($entry.Open())
    try { $mod = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    if ($mod.id -ne 'oozeclient' -or !$mod.version -or
        ($MinecraftVersion -eq '1.21.11' -and $mod.version -ne '0.1.0-rc1+mc1.21.11')) {
        throw 'The mod JAR must be the actual RC1 build.'
    }
} finally { $jarZip.Dispose() }

# Fabric API project version 6qAuTtLR, checked against Fabric's package pin.
$apiUrl = 'https://cdn.modrinth.com/data/P7dR8mSH/versions/6qAuTtLR/fabric-api-0.141.6%2B1.21.11.jar'
$apiHashes = [ordered]@{
    sha1 = 'c98467cbbaf4d197377266795ae015f4130d65b6'
    sha512 = '852d3682c4f353fd0ca86546af8578513e8bcf628377fa9d49c556bc59c5024c69ab56c92fc6dfba11fed19e4316a3f5636e7e07fd3d96f7043b531aef59b637'
}
$apiSize = 2426039
if ($bundle.api_version -ne '0.141.6+1.21.11') {
    $games = [Uri]::EscapeDataString('["' + $MinecraftVersion + '"]')
    $versions = Invoke-RestMethod "https://api.modrinth.com/v2/project/P7dR8mSH/version?game_versions=$games&loaders=%5B%22fabric%22%5D" -UserAgent 'kyluua/Ooze-Client (Prism packaging)' -TimeoutSec 30
    $versions = @($versions | Where-Object { $_.version_number -eq $bundle.api_version -and $_.game_versions -contains $MinecraftVersion -and $_.loaders -contains 'fabric' })
    if ($versions.Count -ne 1) { throw 'Expected exactly one matching Fabric API release on Modrinth.' }
    $file = @($versions[0].files | Where-Object { $_.primary -and $_.filename -like '*.jar' })
    if ($file.Count -ne 1 -or $file[0].url -notmatch '^https://cdn\.modrinth\.com/data/P7dR8mSH/versions/[^/]+/[^/]+\.jar$' -or
        $file[0].hashes.sha1 -notmatch '^[a-f0-9]{40}$' -or $file[0].hashes.sha512 -notmatch '^[a-f0-9]{128}$') {
        throw 'Expected one pinned primary Fabric API JAR.'
    }
    $apiUrl = $file[0].url
    $apiHashes = [ordered]@{sha1=$file[0].hashes.sha1; sha512=$file[0].hashes.sha512}
    $apiSize = $file[0].size
}
$null = New-Item -ItemType Directory -Path $output
$apiFile = Join-Path $output '.fabric-api-check.jar'
try {
    Invoke-WebRequest -Uri $apiUrl -OutFile $apiFile -TimeoutSec 60
    if ((Get-Item -LiteralPath $apiFile).Length -ne $apiSize -or $apiSize -ne $bundle.api.size -or
        (Get-Sha $apiFile SHA1) -ne $apiHashes.sha1 -or (Get-Sha $apiFile SHA512) -ne $apiHashes.sha512 -or
        (Get-Sha $apiFile SHA256) -ne $bundle.api.sha256) {
        throw 'Modrinth Fabric API differs from the pinned release artifact.'
    }
} finally {
    if (Test-Path -LiteralPath $apiFile) { Remove-Item -LiteralPath $apiFile }
}

$packVersion = if ($release.schema -eq 1) { '0.1.0-rc1' } else { "0.1.0-rc1-mc$MinecraftVersion" }
$packName = if ($release.schema -eq 1) { 'Ooze Base RC1' } else { "Ooze Base RC1 ($MinecraftVersion)" }
$index = [ordered]@{
    formatVersion = 1
    game = 'minecraft'
    versionId = $packVersion
    name = $packName
    summary = "Ooze base for Minecraft $MinecraftVersion on Fabric, with Windows x64 native support and Java $java."
    files = @([ordered]@{
        path = "mods/fabric-api-$($bundle.api_version).jar"
        hashes = $apiHashes
        env = [ordered]@{client = 'required'; server = 'unsupported'}
        downloads = @($apiUrl)
        fileSize = $apiSize
    })
    dependencies = [ordered]@{minecraft = $MinecraftVersion; 'fabric-loader' = $loader}
}
$archivePath = Join-Path $output "ooze-base-$packVersion.mrpack"
$jarEntry = 'client-overrides/mods/ooze-client.jar'
$epoch = [DateTimeOffset]::Parse('2026-01-01T00:00:00Z')
$stream = [IO.File]::Open($archivePath, [IO.FileMode]::CreateNew)
try {
    $zip = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $true)
    try {
        $entry = $zip.CreateEntry('modrinth.index.json')
        $entry.LastWriteTime = $epoch
        $writer = [IO.StreamWriter]::new($entry.Open(), [Text.UTF8Encoding]::new($false))
        try { $writer.Write(($index | ConvertTo-Json -Depth 8 -Compress)) } finally { $writer.Dispose() }
        $entry = $zip.CreateEntry($jarEntry)
        $entry.LastWriteTime = $epoch
        $source = [IO.File]::OpenRead($jar.FullName)
        try {
            $destination = $entry.Open()
            try { $source.CopyTo($destination) } finally { $destination.Dispose() }
        } finally { $source.Dispose() }
    } finally { $zip.Dispose() }
} finally { $stream.Dispose() }

# A strict allowlist ensures only the index and our produced client JAR ship.
$zip = [IO.Compression.ZipFile]::OpenRead($archivePath)
try {
    $expected = @($jarEntry, 'modrinth.index.json')
    if ($zip.Entries.Count -ne $expected.Count -or (Compare-Object $expected @($zip.Entries.FullName))) {
        throw 'Unexpected modpack entries.'
    }
    $reader = [IO.StreamReader]::new($zip.GetEntry('modrinth.index.json').Open())
    try { $stored = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    $dep = $stored.files[0]
    if ($stored.formatVersion -ne 1 -or $stored.game -ne 'minecraft' -or $stored.versionId -ne $packVersion -or
        $stored.dependencies.minecraft -ne $MinecraftVersion -or $stored.dependencies.'fabric-loader' -ne $loader -or
        $stored.files.Count -ne 1 -or $dep.path -ne $index.files[0].path -or
        $dep.env.client -ne 'required' -or $dep.env.server -ne 'unsupported' -or
        $dep.fileSize -ne $apiSize -or $dep.hashes.sha1 -ne $apiHashes.sha1 -or
        $dep.hashes.sha512 -ne $apiHashes.sha512 -or $dep.downloads.Count -ne 1 -or $dep.downloads[0] -ne $apiUrl) {
        throw 'Modrinth index validation failed.'
    }
    $embedded = $zip.GetEntry($jarEntry)
    $stream = $embedded.Open()
    try {
        $digest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream))
        if ($embedded.Length -ne $jar.Length -or $digest -ne $bundle.ooze.sha256) { throw 'Embedded JAR changed.' }
    } finally { $stream.Dispose() }
} finally { $zip.Dispose() }

$evidence = [ordered]@{
    archive = [ordered]@{
        file = [IO.Path]::GetFileName($archivePath)
        size = (Get-Item -LiteralPath $archivePath).Length
        sha256 = (Get-Sha $archivePath SHA256)
        sha1 = (Get-Sha $archivePath SHA1)
        sha512 = (Get-Sha $archivePath SHA512)
    }
    ooze = [ordered]@{
        path = $jarEntry
        size = $jar.Length
        sha256 = $bundle.ooze.sha256
        sha1 = (Get-Sha $jar.FullName SHA1)
        sha512 = (Get-Sha $jar.FullName SHA512)
    }
    fabricApi = $index.files[0]
}
$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'mrpack-evidence.json') -Encoding utf8NoBOM
"Validated $($evidence.archive.file) SHA-256 $($evidence.archive.sha256)"
