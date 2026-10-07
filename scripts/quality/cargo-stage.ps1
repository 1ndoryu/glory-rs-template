# Etapa cargo del gate (patrón coolify-manager-rs 119A-1): fija CARGO_TARGET_DIR
# fuera del árbol (C:\tmp), ejecuta el cargo del PATH (el gate exime a la etapa
# del guard vía lease) y, si el subcomando termina en 0, emite el reporte
# findings vacío que exige el gate. En fallo propaga el código de cargo.
# Uso: cargo-stage.ps1 <reportPath> <args de cargo...>
$Report = $args[0]
$cargoArgs = @($args | Select-Object -Skip 1)
$target = $env:GLORY_CARGO_TARGET_DIR
if ([string]::IsNullOrWhiteSpace($target)) { $target = 'C:\tmp\glory-target\nakomi' }
$env:CARGO_TARGET_DIR = $target
& cargo @cargoArgs 2>&1
$code = $LASTEXITCODE
if ($code -eq 0) {
    [System.IO.File]::WriteAllText($Report, '{"schemaVersion":1,"entries":[]}', [System.Text.UTF8Encoding]::new($false))
}
exit $code
