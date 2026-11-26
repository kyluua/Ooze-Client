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
$created=Invoke-Library @('create','Fabric legacy managed','--preset','pvp')
$id=$created.Instance
if($id -notmatch '^[0-9a-f]{32}$'){throw 'Missing real instance identity'}
$vanilla=Invoke-Library @('create','Vanilla legacy custom','--vanilla','--preset','minimal')
$list=Invoke-Library @('list')
if($list.instances.Count -ne 2){throw 'Instance isolation failed'}
$fabric=@($list.instances | Where-Object {$_.instance.id -eq $id})[0]
$legacyManifest=Join-Path (Split-Path -Parent $fabric.game_dir) 'instance.json'
$legacy=Get-Content -LiteralPath $legacyManifest -Raw | ConvertFrom-Json
$legacy.mode='managed'
[IO.File]::WriteAllText($legacyManifest,($legacy | ConvertTo-Json -Depth 100))
$legacyHash=(Get-FileHash -LiteralPath $legacyManifest).Hash
$custom=@($list.instances | Where-Object {$_.instance.id -eq $vanilla.Instance})[0]
$customManifest=Join-Path (Split-Path -Parent $custom.game_dir) 'instance.json'
$customHash=(Get-FileHash -LiteralPath $customManifest).Hash
foreach($attempt in 1..2){
    $snapshot=Invoke-Library @('list')
    if($snapshot.selected -ne $id -or $snapshot.instances.Count -ne 2){
        throw "Reopen $attempt changed the legacy instance identity or selection"
    }
    if((Get-FileHash -LiteralPath $legacyManifest).Hash -ne $legacyHash -or
       (Get-FileHash -LiteralPath $customManifest).Hash -ne $customHash){
        throw "Reopen $attempt rewrote a legacy manifest"
    }
}
foreach($kind in @('mod','resourcepack','datapack','shader')){
    $page=Invoke-Library @('search',$kind)
    if($page.Search.hits.Count -eq 0){throw "No real Modrinth results: $kind"}
    Write-Output "Live Modrinth search passed: $kind ($($page.Search.hits.Count) results)"
}
# Exact owner-reviewed public Iris version for 1.21.11; includes required Sodium.
# Install on the formerly managed Fabric instance without a conversion.
$null=Invoke-Library @('install',$id,'YL57xq9U','ka9PDlsN')
$list=Invoke-Library @('list')
$instance=@($list.instances | Where-Object {$_.instance.id -eq $id})[0]
if($instance.instance.content.Count -ne 2){throw 'Iris/Sodium dependency installation failed'}
$migrated=Get-Content -LiteralPath $legacyManifest -Raw | ConvertFrom-Json
if($migrated.mode -ne 'custom' -or $migrated.id -ne $id -or $list.selected -ne $id){
    throw 'Content mutation failed to preserve identity and normalize the legacy manifest'
}
$verify=Invoke-Library @('verify',$id)
if($verify.Integrity.issues.Count -ne 0 -or $verify.Integrity.verified_files -ne 2){throw 'Installed hashes failed verification'}
$null=Invoke-Library @('disable',$id,'YL57xq9U')
$null=Invoke-Library @('enable',$id,'YL57xq9U')
$installed=Invoke-Library @('list')
$target=@($installed.instances|Where-Object {$_.instance.id -eq $id})[0]
$iris=@($target.instance.content|Where-Object project -eq 'YL57xq9U')[0]
[IO.File]::AppendAllText((Join-Path $target.game_dir $iris.path),'controlled modified-file fixture')
$verify=Invoke-Library @('verify',$id)
if($verify.Integrity.issues.Count -eq 0){throw 'Changed registered content was not detected'}
$null=Invoke-Library @('repair',$id,'YL57xq9U')
$verify=Invoke-Library @('verify',$id)
if($verify.Integrity.issues.Count -ne 0){throw 'Explicit content repair failed'}
$profile=Invoke-Library @('export-profile',$id)
if($profile.schema -ne 'ooze.profile/1' -or $profile.base -ne 'pvp'){throw 'Canonical profile schema was not persisted'}
$appearance=Invoke-Library @('appearance')
if($appearance.schema -ne 'ooze.theme/1'){throw 'Canonical global appearance schema was not persisted'}
$worlds=Invoke-Library @('worlds',$id)
if($null -eq $worlds.Worlds -or $worlds.Worlds.worlds.Count -ne 0){throw 'Fresh instance should contain no worlds'}
$unregistered=Join-Path $fabric.game_dir 'mods/unregistered.jar'
[IO.File]::WriteAllText($unregistered,'controlled integrity fixture')
$extraHash=(Get-FileHash -LiteralPath $unregistered).Hash
$verify=Invoke-Library @('verify',$id)
if($verify.Integrity.issues.Count -ne 0 -or $verify.Integrity.verified_files -ne 2){
    throw 'Verify rejected unregistered content or failed to check the receipts'
}
if((Get-FileHash -LiteralPath $unregistered).Hash -ne $extraHash){throw 'Verify changed unregistered content'}
Write-Output 'Repeated legacy migration, four live searches, Iris/Sodium install, receipt repair and player content preservation passed.'
