[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$DataDirectory,
    [ValidateSet('1.21.11','26.1','26.1.1','26.1.2','26.2','26.3')][string]$MinecraftVersion = '1.21.11'
)
$ErrorActionPreference='Stop'
$cli=Join-Path $PackageDirectory 'ooze-launcher-cli.exe'
function Run-Cli([string[]]$Arguments){
    $lines=& $cli --data-dir $DataDirectory @Arguments
    if($LASTEXITCODE){throw "Target command failed: $($Arguments -join ' ')"}
    return ($lines -join "`n")
}
foreach($target in @('fabric','vanilla')){
    $arguments=@('library','create',"CI $target $MinecraftVersion",'--minecraft',$MinecraftVersion)
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
    if($hasOoze -ne $instance.instance.target.ooze){throw 'Wrong Ooze distribution selection'}
    $minecraftTarget=$instance.instance.target
    $expectedVersion=if($target -eq 'fabric'){"fabric-loader-$($minecraftTarget.loader_version)-$MinecraftVersion"}else{$MinecraftVersion}
    if($receipt.version_name -ne $expectedVersion -or $minecraftTarget.minecraft -ne $MinecraftVersion){throw 'Wrong Minecraft release'}
    $expectedJava=if($MinecraftVersion -eq '1.21.11'){21}else{25}
    if($minecraftTarget.java_major -ne $expectedJava -or !$receipt.managed_java){throw 'Wrong Java selection'}
    $verify=(Run-Cli @('library','verify',$id))|ConvertFrom-Json
    if(-not $verify.Integrity.baseline_verified -or $verify.Integrity.issues.Count -ne 0){throw 'Installed game files did not verify'}
    $fixtures=@('options.txt','saves/test/level.dat','saves/test/datapacks/test.zip','screenshots/test.png','resourcepacks/test.zip','shaderpacks/test.zip','mods/unregistered.jar')
    $before=@{}
    foreach($relative in $fixtures){
        $path=Join-Path $instance.game_dir $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
        [IO.File]::WriteAllText($path,"controlled repair fixture: $relative")
        $before[$relative]=(Get-FileHash -LiteralPath $path).Hash
    }
    $generation=Join-Path $instance.game_dir ".ooze-launcher/$($receipt.directory)"
    $artifact=if($hasOoze){'mods/ooze-client.jar'}else{'client.jar'}
    [IO.File]::AppendAllText((Join-Path $generation $artifact),'controlled game-file corruption')
    $verify=(Run-Cli @('library','verify',$id))|ConvertFrom-Json
    if($verify.Integrity.baseline_verified -or $verify.Integrity.issues.Count -ne 1){throw 'Corrupt packaged game file did not fail verification'}
    $null=Run-Cli @('instance',$id,'repair')
    foreach($relative in $fixtures){
        $path=Join-Path $instance.game_dir $relative
        if((Get-FileHash -LiteralPath $path).Hash -ne $before[$relative]){
            throw "Repair changed $relative"
        }
    }
    $verify=(Run-Cli @('library','verify',$id))|ConvertFrom-Json
    if(-not $verify.Integrity.baseline_verified -or $verify.Integrity.issues.Count -ne 0){throw 'Repaired game files did not verify'}
    Write-Output "Actual Java $expectedJava install/repair passed: $target $MinecraftVersion (no live game/auth claim)"
}
