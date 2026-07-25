param(
    [string[]]$TargetDirs = @(),
    [string[]]$ExcludeDirs = @(),
    [int]$MaxTotalMB = 15360,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$projectRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$sharedScript = Join-Path $projectRoot 'glory-rs\scripts\clean-cargo-target.ps1'

if (-not (Test-Path $sharedScript)) {
    throw "No se encontro el script compartido: $sharedScript"
}

$resolvedTargetDirs = @()
if ($TargetDirs.Count -gt 0) {
    $resolvedTargetDirs += $TargetDirs
} elseif ($env:CARGO_TARGET_DIR) {
    $resolvedTargetDirs += $env:CARGO_TARGET_DIR
} else {
    $resolvedTargetDirs += 'C:\tmp\glory-target'
}

$argsList = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', $sharedScript,
    '-TargetDirs'
) + $resolvedTargetDirs

if ($ExcludeDirs.Count -gt 0) {
    $argsList += @('-ExcludeDirs') + $ExcludeDirs
}

$argsList += @(
    '-MaxTotalMB', $MaxTotalMB
)

if ($Force) {
    $argsList += '-Force'
}

& powershell @argsList
exit $LASTEXITCODE
