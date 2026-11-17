[CmdletBinding()]
param([Parameter(Mandatory)][string]$CliExe,[Parameter(Mandatory)][string]$DataDirectory)
$ErrorActionPreference='Stop'
$cli=(Resolve-Path -LiteralPath $CliExe).Path
if(Test-Path -LiteralPath $DataDirectory){throw 'Use a fresh test library directory'}
function Invoke-Library([string[]]$Arguments){
    $lines=& $cli --data-dir $DataDirectory library @Arguments
    if($LASTEXITCODE){throw "Library command failed: $($Arguments[0])"}
    ($lines -join "`n") | ConvertFrom-Json
}
$created=Invoke-Library @('create','Fabric custom','--preset','pvp')
$id=$created.Instance
if($id -notmatch '^[0-9a-f]{32}$'){throw 'Missing real instance identity'}
$vanilla=Invoke-Library @('create','Vanilla managed','--vanilla','--managed','--preset','minimal')
$list=Invoke-Library @('list')
if($list.instances.Count -ne 2){throw 'Instance isolation failed'}
foreach($kind in @('mod','resourcepack','datapack','shader')){
    $page=Invoke-Library @('search',$kind)
    if($page.Search.hits.Count -eq 0){throw "No real Modrinth results: $kind"}
    Write-Output "Live Modrinth search passed: $kind ($($page.Search.hits.Count) results)"
}
# Exact owner-reviewed public Iris version for 1.21.11; includes required Sodium.
# This is user-requested Custom content, not a Managed approval or bundled dependency.
$null=Invoke-Library @('install',$id,'YL57xq9U','ka9PDlsN')
$list=Invoke-Library @('list')
$instance=@($list.instances | Where-Object {$_.instance.id -eq $id})[0]
if($instance.instance.content.Count -ne 2){throw 'Iris/Sodium dependency installation failed'}
$verify=Invoke-Library @('verify',$id)
if($verify.Integrity.issues.Count -ne 0 -or $verify.Integrity.verified_files -ne 2){throw 'Installed hashes failed verification'}
$null=Invoke-Library @('disable',$id,'YL57xq9U')
$null=Invoke-Library @('enable',$id,'YL57xq9U')
$profile=Invoke-Library @('export-profile',$id)
if($profile.schema -ne 'ooze.profile/1' -or $profile.base -ne 'pvp'){throw 'Canonical profile schema was not persisted'}
$appearance=Invoke-Library @('appearance')
if($appearance.schema -ne 'ooze.theme/1'){throw 'Canonical global appearance schema was not persisted'}
$worlds=Invoke-Library @('worlds',$id)
if($null -eq $worlds.Worlds -or $worlds.Worlds.worlds.Count -ne 0){throw 'Fresh instance should contain no worlds'}
$managed=@($list.instances | Where-Object {$_.instance.id -eq $vanilla.Instance})[0]
$mods=Join-Path $managed.game_dir 'mods'
New-Item -ItemType Directory -Path $mods | Out-Null
[IO.File]::WriteAllText((Join-Path $mods 'unapproved.jar'),'controlled integrity fixture')
$verify=Invoke-Library @('verify',$vanilla.Instance)
if($verify.Integrity.issues.Count -eq 0){throw 'Managed extra content was not detected'}
if(-not (Test-Path -LiteralPath (Join-Path $mods 'unapproved.jar'))){throw 'Integrity check deleted player content'}
Write-Output 'Library real CLI, four live search types, Iris/Sodium install, disable/enable, profile persistence and managed tamper detection passed.'
