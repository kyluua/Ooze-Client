[CmdletBinding()]
param([Parameter(Mandatory)][string]$PackageDirectory,[Parameter(Mandatory)][string]$DataDirectory)
$ErrorActionPreference='Stop'
$cli=Join-Path $PackageDirectory 'ooze-launcher-cli.exe'
function Run-Cli([string[]]$Arguments){
    $lines=& $cli --data-dir $DataDirectory @Arguments
    if($LASTEXITCODE){throw "Target command failed: $($Arguments -join ' ')"}
    return ($lines -join "`n")
}
foreach($target in @('fabric','vanilla')){
    $arguments=@('library','create',"CI $target",'--managed')
    if($target -eq 'vanilla'){$arguments+='--vanilla'}
    $created=(Run-Cli $arguments)|ConvertFrom-Json
    $id=$created.Instance
    $null=Run-Cli @('instance',$id,'install')
    $list=(Run-Cli @('library','list'))|ConvertFrom-Json
    $instance=@($list.instances|Where-Object {$_.instance.id -eq $id})[0]
    $receipt=Get-Content -LiteralPath (Join-Path $instance.game_dir '.ooze-launcher/active-install.json') -Raw | ConvertFrom-Json
    $expected=if($target -eq 'fabric'){'net.fabricmc.loader.impl.launch.knot.KnotClient'}else{'net.minecraft.client.main.Main'}
    if($receipt.main_class -ne $expected){throw 'Wrong loader entrypoint'}
    $hasOoze=@($receipt.files|Where-Object path -eq 'mods/ooze-client.jar').Count -eq 1
    if($hasOoze -ne ($target -eq 'fabric')){throw 'Wrong Ooze distribution selection'}
    $verify=(Run-Cli @('library','verify',$id))|ConvertFrom-Json
    if(-not $verify.Integrity.baseline_verified -or $verify.Integrity.issues.Count -ne 0){throw 'Managed baseline did not verify'}
    $options=Join-Path $instance.game_dir 'options.txt'
    [IO.File]::WriteAllText($options,'controlled player settings fixture')
    $before=(Get-FileHash -LiteralPath $options).Hash
    $null=Run-Cli @('instance',$id,'repair')
    if((Get-FileHash -LiteralPath $options).Hash -ne $before){throw 'Repair changed options.txt'}
    Write-Output "Actual Java 21 install/repair and baseline verification passed: $target 1.21.11 (no live game/auth claim)"
}
