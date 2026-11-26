[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BasePackageDirectory,
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$DataDirectory
)

$ErrorActionPreference = 'Stop'
$legacy = Join-Path $BasePackageDirectory 'ooze-launcher-cli.exe'
$candidate = Join-Path $PackageDirectory 'ooze-launcher-cli.exe'
function Invoke-Library($Cli, [string[]]$Arguments) {
    $lines = & $Cli --data-dir $DataDirectory library @Arguments
    if ($LASTEXITCODE) { throw "Library command failed: $($Arguments[0])" }
    ($lines -join "`n") | ConvertFrom-Json -Depth 100
}

$before = Invoke-Library $legacy @('list')
if (!$before.selected -or $before.instances.Count -lt 2) { throw 'Expected the seeded 1.21.11 install fixtures.' }
$targets = @(Invoke-Library $candidate @('targets'))
foreach ($version in @('1.21.11','26.1','26.1.1','26.1.2','26.2','26.3')) {
    foreach ($loader in @('vanilla','fabric')) {
        $expectedJava = if ($version -eq '1.21.11') { 21 } else { 25 }
        $matching = @($targets | Where-Object {$_.minecraft -eq $version -and $_.loader -eq $loader -and !$_.ooze})
        if ($matching.Count -ne 1 -or $matching[0].java_major -ne $expectedJava) {
            throw "Wrong candidate availability for $version $loader."
        }
    }
}
$after = Invoke-Library $candidate @('list')
if (($before | ConvertTo-Json -Depth 100 -Compress) -ne ($after | ConvertTo-Json -Depth 100 -Compress)) {
    throw 'Opening the catalog changed legacy instances or selection.'
}
$new = Invoke-Library $candidate @('create','Separate Fabric 26.3','--minecraft','26.3','--plain-fabric')
$after = Invoke-Library $candidate @('list')
$created = @($after.instances | Where-Object {$_.instance.id -eq $new.Instance})
if ($after.selected -ne $before.selected -or $created.Count -ne 1 -or $created[0].instance.target.minecraft -ne '26.3' -or
    $created[0].instance.target.ooze -or $created[0].game_dir -in $before.instances.game_dir) {
    throw 'Creating a new version changed selection or reused a legacy game directory.'
}
$prior = @($after.instances | Where-Object {$_.instance.id -ne $new.Instance})
if ((ConvertTo-Json -InputObject $prior -Depth 100 -Compress) -ne (ConvertTo-Json -InputObject $before.instances -Depth 100 -Compress)) {
    throw 'Creating a new version changed an existing instance.'
}
Write-Output 'Exact target catalog, selected legacy instance and separate new game directory verified.'
