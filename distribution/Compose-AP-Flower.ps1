[CmdletBinding()]
param([Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$GameDir,[Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$Witchy,[string]$MarioDir)
# Windows PowerShell binds advanced-script parameter defaults before PSScriptRoot is populated.
if ([string]::IsNullOrWhiteSpace($MarioDir)) { $MarioDir = $PSScriptRoot }
$ErrorActionPreference='Stop'
if(Get-CimInstance Win32_Process -Filter "Name = 'eldenring.exe'"){throw 'Save and quit Elden Ring before composing menu atlases.'}
$python=Get-Command py -ErrorAction SilentlyContinue
if(-not $python){$python=Get-Command python -ErrorAction SilentlyContinue}
if(-not $python){throw 'Install Python 3.11 or later first.'}
$game=(Resolve-Path -LiteralPath $GameDir).Path
$mario=(Resolve-Path -LiteralPath $MarioDir).Path
$witchyPath=(Resolve-Path -LiteralPath $Witchy).Path
$menu=Join-Path $mario 'package\menu'
foreach($quality in @('hi','low')){if(-not(Test-Path -LiteralPath (Join-Path $menu "$quality\01_common.tpf.dcx"))){throw 'Launch er-mario-setup.me3 with your own US SM64 ROM once and let setup finish first.'}}
$oodle=Join-Path $game 'oo2core_6_win64.dll'
if(-not(Test-Path -LiteralPath $oodle)){throw 'GameDir must contain eldenring.exe and its oo2core_6_win64.dll.'}
$work=Join-Path $mario ('.local-build\flower-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
$layouts=Join-Path $work 'layouts\menu'
$out=Join-Path $work 'combined\menu'
& $python.Source (Join-Path $PSScriptRoot 'tools\extract_local_layouts.py') --game $game --output $layouts
if($LASTEXITCODE -ne 0){throw 'Local layout extraction failed.'}
& $python.Source (Join-Path $PSScriptRoot 'tools\build_ap_icon.py') --menu $menu --layout-menu $layouts --payload (Join-Path $PSScriptRoot 'tools\ap_icon_src\ap_flower_160.bc7') --witchy $witchyPath --oodle $oodle --bundles hi,low --icon-id 92 --out $out --work (Join-Path $work 'work')
if($LASTEXITCODE -ne 0){throw 'Atlas composition failed; installed menu unchanged.'}
foreach($quality in @('hi','low')) {
    $source=Join-Path $out "$quality\01_common.tpf.dcx"
    if(-not(Test-Path -LiteralPath $source)){throw 'Composition did not produce both atlases.'}
}
foreach($quality in @('hi','low')) {
    $target=Join-Path $menu "$quality\01_common.tpf.dcx"
    $backup=Join-Path $work "backup\$quality"
    New-Item -ItemType Directory -Path $backup -Force | Out-Null
    Copy-Item -LiteralPath $target -Destination $backup
    Copy-Item -LiteralPath (Join-Path $out "$quality\01_common.tpf.dcx") -Destination $target -Force
}
Write-Host "Combined flower and Mario icons installed locally; backups: $work\backup"
Write-Host 'Never redistribute package, .local-build, ROM, or extracted layout files.'
