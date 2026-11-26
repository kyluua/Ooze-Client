[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$Compiler
)

$ErrorActionPreference = 'Stop'
$payload = (Resolve-Path -LiteralPath $PackageDirectory).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) {
    throw 'Use a new installer output directory.'
}

if (!$Compiler) {
    $command = Get-Command ISCC.exe -CommandType Application -ErrorAction SilentlyContinue
    $Compiler = if ($command) { $command.Source } else {
        Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'
    }
}
$iscc = (Resolve-Path -LiteralPath $Compiler).Path

$checksums = Join-Path $payload 'SHA256SUMS'
$checked = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($line in Get-Content -LiteralPath $checksums) {
    if ($line -notmatch '^([a-f0-9]{64})  ([^\\:/]+(?:/[^\\:/]+)*)$') {
        throw 'Invalid entry in the launcher package checksum list.'
    }
    $expected = $Matches[1]
    $relative = $Matches[2]
    if ($relative -match '(^|/)\.{1,2}(/|$)' -or !$checked.Add($relative)) {
        throw "Invalid or duplicate launcher package path: $relative"
    }
    $file = Join-Path $payload $relative
    if ((Get-FileHash -LiteralPath $file).Hash -ine $expected) {
        throw "Launcher package checksum mismatch: $relative"
    }
}
foreach ($required in @('ooze-launcher.exe','ooze-launcher-cli.exe','ooze-client.jar','release.json')) {
    if (!$checked.Contains($required)) { throw "Missing launcher package file: $required" }
}
foreach ($file in Get-ChildItem -LiteralPath $payload -Recurse -Force) {
    if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'Launcher installer input cannot contain links.'
    }
    if (!$file.PSIsContainer) {
        $relative = [IO.Path]::GetRelativePath($payload, $file.FullName).Replace('\','/')
        if ($relative -ne 'SHA256SUMS' -and !$checked.Contains($relative)) {
            throw "Unlisted launcher package file: $relative"
        }
    }
}

$null = New-Item -ItemType Directory -Path $output
& $iscc "/DPayloadDirectory=$payload" "/O$output" (Join-Path $PSScriptRoot 'ooze-launcher.iss')
if ($LASTEXITCODE) { throw "Inno Setup compiler exited with $LASTEXITCODE." }
$setup = Get-Item -LiteralPath (Join-Path $output 'Ooze-Launcher-0.1.0-rc1-Windows-x64-Setup.exe')
$line = (Get-FileHash -LiteralPath $setup.FullName).Hash.ToLowerInvariant() + '  ' + $setup.Name
Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS') -Value $line -Encoding utf8NoBOM
