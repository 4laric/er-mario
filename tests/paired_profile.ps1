$ErrorActionPreference='Stop'
$helper=Join-Path $PSScriptRoot '../distribution/Create-Paired-Profile.ps1'
$tempBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixture=Join-Path $tempBase ('er-mario-profile-'+[Guid]::NewGuid().ToString('N'))
if(-not([IO.Path]::GetFullPath($fixture).StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase))){throw 'Fixture must stay under the temporary directory.'}
function Assert($condition,[string]$message){if(-not $condition){throw $message}}
try {
    $companion=Join-Path $fixture 'Mario companion'
    $bundle=Join-Path $fixture 'AP bundle with spaces'
    New-Item -ItemType Directory -Path $companion,$bundle | Out-Null
    $copy=Join-Path $companion 'Create-Paired-Profile.ps1'
    Copy-Item -LiteralPath $helper -Destination $copy
    $files=@('eldenring_archipelago.dll','MapForGoblins.dll','MapForGoblins.upstream.dll','MapForGoblins.ini','MapForGoblins.AP.ini')
    foreach($file in $files){[IO.File]::WriteAllText((Join-Path $bundle $file),'fixture')}
    & $copy -APClientDir $bundle -SaveFile 'ER_custom_mario.sl2'
    $profile=Join-Path $companion 'er-mario-ap.me3'
    $text=[IO.File]::ReadAllText($profile)
    $natives=@([regex]::Matches($text,'(?m)^path = ("(?:\\.|[^"\\])*")\r?$') | ForEach-Object {$_.Groups[1].Value | ConvertFrom-Json})
    Assert ($natives.Count -eq 4) 'Expected three native paths and one package path.'
    Assert ($natives[0] -eq 'er_mario.dll') 'Mario native missing.'
    Assert ($natives[1] -eq (Join-Path $bundle 'eldenring_archipelago.dll')) 'AP absolute native path missing.'
    Assert ($natives[2] -eq (Join-Path $bundle 'MapForGoblins.dll')) 'Map adapter absolute native path missing.'
    Assert ($natives[3] -eq 'package') 'Composed Mario package missing.'
    Assert ($text.Contains('savefile = "ER_custom_mario.sl2"')) 'Custom save must be preserved.'
    Assert (-not $text.Contains('flower-package')) 'Normal flower atlas would overwrite composed Mario icons.'
    Assert (-not $text.Contains('MapForGoblins.upstream.dll')) 'Adapter must own upstream loading.'
    Assert ([regex]::Matches($text,'\[\[packages\]\]').Count -eq 1) 'Only the composed Mario package should load.'
    & $copy -APClientDir $bundle
    Assert ([IO.File]::ReadAllText($profile).Contains('savefile = "ER0000_mario_ap.sl2"')) 'Default save changed.'
    $backup=@(Get-ChildItem -LiteralPath $companion -Filter 'er-mario-ap.me3.bak-*')
    Assert ($backup.Count -eq 1) 'Existing profile must be backed up.'
    Assert ([IO.File]::ReadAllText($backup[0].FullName) -eq $text) 'Backup must preserve the previous profile.'
    $good=[IO.File]::ReadAllText($profile)
    foreach($file in $files){
        $dependency=Join-Path $bundle $file
        Remove-Item -LiteralPath $dependency
        $failure=$null
        try {& $copy -APClientDir $bundle} catch {$failure=$_.Exception.Message}
        Assert ($failure -and $failure.Contains($file)) "Missing dependency must name $file."
        Assert ([IO.File]::ReadAllText($profile) -eq $good) 'Dependency failure must leave the existing profile intact.'
        [IO.File]::WriteAllText($dependency,'fixture')
    }
    $failure=$null
    try {& $copy -APClientDir $bundle -SaveFile '../bad.sl2'} catch {$failure=$_.Exception.Message}
    Assert ($failure -eq 'SaveFile must be a filename.') 'Reject a save path instead of a filename.'
    Assert ([IO.File]::ReadAllText($profile) -eq $good) 'Invalid save must leave the profile intact.'
    Write-Host 'PASS paired profile: three natives, composed atlas only, saves/backups, all five missing dependencies and invalid save.'
} finally {
    if(Test-Path -LiteralPath $fixture){Remove-Item -LiteralPath $fixture -Recurse -Force}
}
