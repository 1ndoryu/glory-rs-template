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
$hiddenRunner = Join-Path $PSScriptRoot 'run-cargo-cleanup-hidden.vbs'
if (-not (Test-Path -LiteralPath $cleanScript)) {
    throw "No se encontro el limpiador compartido: $cleanScript"
}
if (-not (Test-Path -LiteralPath $hiddenRunner)) {
    throw "No se encontro el lanzador oculto: $hiddenRunner"
}

<# [257A-6] La limpieza no puede depender del launcher dev: cargo check/test
 también escriben en el target compartido y antes podían llevarlo a 22+ GB
 cuando el watcher terminaba junto con npm run dev. La tarea periódica usa el
 limpiador conservador, que se aplaza mientras cargo/rustc están activos.
 [257A-7] WindowStyle Hidden evita que la ejecución periódica interrumpa al
 usuario mostrando una consola cada dos minutos.
 [257A-8] powershell.exe puede mostrar un destello antes de procesar
 WindowStyle. wscript.exe es un proceso sin consola y lanza el limpiador con
 ventana 0 desde el primer instante, sin requerir privilegios administrativos. #>
$wscriptExe = Join-Path $env:WINDIR 'System32\wscript.exe'
$arguments = "//B //Nologo `"$hiddenRunner`" `"$cleanScript`" `"$TargetDir`" $MaxTotalMB"
$action = New-ScheduledTaskAction -Execute $wscriptExe -Argument $arguments
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
