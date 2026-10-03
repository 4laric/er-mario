[CmdletBinding()]
param([Parameter(Mandatory)][string]$APClientDir,[string]$SaveFile='ER0000_mario_ap.sl2')
$ErrorActionPreference='Stop'
$bundle=(Resolve-Path -LiteralPath $APClientDir).Path
$client=Join-Path $bundle 'eldenring_archipelago.dll'
if(-not(Test-Path -LiteralPath $client -PathType Leaf)){throw 'APClientDir must be the normal AP bundle me3 folder containing eldenring_archipelago.dll.'}
$mapFiles=@('MapForGoblins.dll','MapForGoblins.upstream.dll','MapForGoblins.ini','MapForGoblins.AP.ini')
$missing=@($mapFiles | Where-Object {-not(Test-Path -LiteralPath (Join-Path $bundle $_) -PathType Leaf)})
if($missing.Count){throw "APClientDir is missing MapForGoblins dependencies: $($missing -join ', '). Extract the complete normal AP bundle and point at its me3 folder; keep both map DLLs and both INI files together."}
if([IO.Path]::GetFileName($SaveFile) -ne $SaveFile){throw 'SaveFile must be a filename.'}
$clientToml=ConvertTo-Json -InputObject $client -Compress
$mapToml=ConvertTo-Json -InputObject (Join-Path $bundle 'MapForGoblins.dll') -Compress
$saveToml=ConvertTo-Json -InputObject $SaveFile -Compress
$profile=@"
profileVersion = "v1"
savefile = $saveToml
disable_arxan = true
mem_patch = false
[[supports]]
game = "eldenring"
[[natives]]
path = "er_mario.dll"
[[natives]]
path = $clientToml
[[natives]]
path = $mapToml
[[packages]]
id = "er-mario"
path = "package"
"@
$profilePath=Join-Path $PSScriptRoot 'er-mario-ap.me3'
if(Test-Path -LiteralPath $profilePath){Copy-Item -LiteralPath $profilePath -Destination ($profilePath+'.bak-'+(Get-Date -Format 'yyyyMMdd-HHmmss-ffff'))}
[IO.File]::WriteAllText($profilePath,$profile,[Text.UTF8Encoding]::new($false))
Write-Host 'Created er-mario-ap.me3 with Mario, AP and MapForGoblins. AP and map settings stay beside their existing DLLs.'
