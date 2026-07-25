param(
    [string]$TargetDir = 'C:\tmp\glory-target',
    [int]$MaxTotalMB = 15360,
    [int]$IntervalMinutes = 2,
    [string]$TaskName = 'GloryCargoTargetCleanup'
)

$ErrorActionPreference = 'Stop'

if ($MaxTotalMB -le 0) {
    throw 'MaxTotalMB debe ser mayor que cero.'
}
if ($IntervalMinutes -le 0) {
    throw 'IntervalMinutes debe ser mayor que cero.'
}

$projectRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$cleanScript = Join-Path $projectRoot 'glory-rs\scripts\clean-cargo-target.ps1'
if (-not (Test-Path -LiteralPath $cleanScript)) {
    throw "No se encontro el limpiador compartido: $cleanScript"
}

<# [257A-6] La limpieza no puede depender del launcher dev: cargo check/test
 también escriben en el target compartido y antes podían llevarlo a 22+ GB
 cuando el watcher terminaba junto con npm run dev. La tarea periódica usa el
 limpiador conservador, que se aplaza mientras cargo/rustc están activos. #>
$powershellExe = Join-Path $PSHOME 'powershell.exe'
$arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$cleanScript`" -TargetDirs `"$TargetDir`" -MaxTotalMB $MaxTotalMB"
$action = New-ScheduledTaskAction -Execute $powershellExe -Argument $arguments
$trigger = New-ScheduledTaskTrigger `
    -Once `
    -At (Get-Date).AddMinutes(1) `
    -RepetitionInterval (New-TimeSpan -Minutes $IntervalMinutes) `
    -RepetitionDuration (New-TimeSpan -Days 3650)
$settings = New-ScheduledTaskSettingsSet `
    -StartWhenAvailable `
    -MultipleInstances IgnoreNew `
    -ExecutionTimeLimit (New-TimeSpan -Minutes 10)

Register-ScheduledTask `
    -TaskName $TaskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Description "Mantiene $TargetDir por debajo de $MaxTotalMB MB cuando Cargo esta inactivo." `
    -Force | Out-Null

Start-ScheduledTask -TaskName $TaskName
Write-Host "[cargo-clean] tarea $TaskName registrada cada $IntervalMinutes min; limite=$MaxTotalMB MB"
