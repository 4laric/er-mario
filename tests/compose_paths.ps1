# Regression for advanced-script defaults under Windows PowerShell 5.1.
$ErrorActionPreference = 'Stop'
$script = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../distribution/Compose-AP-Flower.ps1')).Path
$windowsPowerShell = (Get-Command powershell.exe).Source
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('Mario path test ' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $fixture | Out-Null
try {
    foreach ($explicit in @($false, $true)) {
        $arguments = @('-NoProfile', '-File', $script, '-GameDir', $env:WINDIR, '-Witchy', $windowsPowerShell)
        if ($explicit) { $arguments += @('-MarioDir', $fixture) }
        $ErrorActionPreference = 'Continue'
        $output = (& $windowsPowerShell @arguments 2>&1 | Out-String)
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        if ($code -eq 0 -or $output -notmatch 'Launch er-mario-setup.me3') {
            throw "Path preflight failed (explicit=$explicit): $output"
        }
        if ($output -match 'empty string') { throw 'An empty path reached Resolve-Path.' }
    }
    Write-Host 'Composition path defaults and explicit paths with spaces passed on Windows PowerShell.'
} finally {
    Remove-Item -LiteralPath $fixture
}
