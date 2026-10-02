[CmdletBinding()]
param([Parameter(Mandatory)][string]$APClientDir,[string]$SaveFile='ER0000_mario_ap.sl2')
$ErrorActionPreference='Stop'
$client=Join-Path (Resolve-Path -LiteralPath $APClientDir).Path 'eldenring_archipelago.dll'
if(-not(Test-Path -LiteralPath $client -PathType Leaf)){throw 'APClientDir must be the normal AP bundle me3 folder containing eldenring_archipelago.dll.'}
if([IO.Path]::GetFileName($SaveFile) -ne $SaveFile){throw 'SaveFile must be a filename.'}
$clientToml=ConvertTo-Json -InputObject $client -Compress
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
[[packages]]
id = "er-mario"
path = "package"
"@
$profilePath=Join-Path $PSScriptRoot 'er-mario-ap.me3'
if(Test-Path -LiteralPath $profilePath){Copy-Item -LiteralPath $profilePath -Destination ($profilePath+'.bak-'+(Get-Date -Format 'yyyyMMdd-HHmmss-ffff'))}
[IO.File]::WriteAllText($profilePath,$profile,[Text.UTF8Encoding]::new($false))
Write-Host 'Created er-mario-ap.me3. AP reads its configuration beside its existing DLL.'
