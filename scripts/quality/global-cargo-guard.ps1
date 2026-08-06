<#
[028A-6 Fase 5] LEGACY — adaptador de migración del guard. El runtime global
    de Sentinel (sentinel install, %LOCALAPPDATA%\GlorySentinel) ya genera su
    propio guard de PowerShell; este archivo del repo se conserva SOLO para
    ramas antiguas y se retirará tras dos releases con rollback probado (plan
    028A-6 Fase 5). No modificar su lógica: la fuente canónica es el runtime.

.SYNOPSIS
    PowerShell interceptor for expensive Cargo commands.

.DESCRIPTION
    The project quality gate is still the source of truth. This profile also
    routes direct frontend validation tools through the same command policy.
    Non-Glory projects and development commands pass through unchanged.
#>

$qualityCommandGuardScript = Join-Path $PSScriptRoot 'quality-command-guard.mjs'

function Find-GloryQualityRoot {
    param([string]$StartPath = (Get-Location).Path)
    $candidate = [System.IO.Path]::GetFullPath($StartPath)
    while ($candidate) {
        if ((Test-Path (Join-Path $candidate 'quality.config.json')) -and
            (Test-Path (Join-Path $candidate 'scripts\quality\heavy-run-guard.mjs'))) {
            return $candidate
        }
        $parent = Split-Path -Parent $candidate
        if (-not $parent -or $parent -eq $candidate) { break }
        $candidate = $parent
    }
    return $null
}

function Invoke-GloryQualityCommandGuard {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
    )
    $qualityRoot = Find-GloryQualityRoot
    if (-not $qualityRoot) { return 0 }
    $node = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    & $node $qualityCommandGuardScript --project-root $qualityRoot --executable $Executable -- @Arguments
    return $LASTEXITCODE
}

function Resolve-GloryExternalCommand {
    param([Parameter(Mandatory = $true)][string]$Name, [string]$ConfiguredVariable)
    if ($ConfiguredVariable) {
        $configured = [Environment]::GetEnvironmentVariable($ConfiguredVariable, 'Process')
        if ($configured -and (Test-Path -LiteralPath $configured)) { return $configured }
        $configured = [Environment]::GetEnvironmentVariable($ConfiguredVariable, 'User')
        if ($configured -and (Test-Path -LiteralPath $configured)) { return $configured }
    }
    $shimPath = Join-Path $PSScriptRoot "$Name.cmd"
    $command = Get-Command "$Name.cmd" -CommandType Application -ErrorAction Stop |
        Where-Object { $_.Source -ne $shimPath } |
        Select-Object -First 1
    if (-not $command) { throw "No se encontró el ejecutable real de $Name" }
    return $command.Source
}

function Get-GloryNpmForwardArguments {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    $values = @($Arguments)
    $runIndex = -1
    for ($index = 0; $index -lt $values.Count; $index++) {
        if ($values[$index] -in @('run', 'run-script')) { $runIndex = $index; break }
    }
    if ($runIndex -lt 0) { return $values }
    $separatorIndex = -1
    for ($index = 0; $index -lt $values.Count; $index++) {
        if ($values[$index] -eq '--') { $separatorIndex = $index; break }
    }
    $scriptArgumentIndex = $runIndex + 2
    if ($separatorIndex -ge 0 -or $values.Count -le $scriptArgumentIndex) { return $values }
    return @(
        $values[0..($runIndex + 1)]
        '--'
        $values[$scriptArgumentIndex..($values.Count - 1)]
    )
}

function cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'cargo' -Arguments $CargoArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realCargo = (Get-Command cargo.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $qualityRoot = Find-GloryQualityRoot
    $command = $CargoArguments | Where-Object { $_ -notlike '-*' } | Select-Object -First 1
    $isHeavy = $command -in @('test', 'clippy', 'bench')
    if (-not $qualityRoot -or -not $isHeavy) {
        & $realCargo @CargoArguments
        return $LASTEXITCODE
    }

    $node = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $guard = Join-Path $qualityRoot 'scripts\quality\heavy-run-guard.mjs'
    & $node $guard --execute-cargo --project-root $qualityRoot --cargo-path $realCargo -- @CargoArguments
    return $LASTEXITCODE
}

function npm {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$NpmArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'npm' -Arguments $NpmArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realNpm = Resolve-GloryExternalCommand -Name 'npm' -ConfiguredVariable 'GLORY_REAL_NPM'
    $forwardArguments = Get-GloryNpmForwardArguments -Arguments $NpmArguments
    & $realNpm @forwardArguments
    return $LASTEXITCODE
}

function npx {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$NpxArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'npx' -Arguments $NpxArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realNpx = Resolve-GloryExternalCommand -Name 'npx' -ConfiguredVariable 'GLORY_REAL_NPX'
    & $realNpx @NpxArguments
    return $LASTEXITCODE
}

function node {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$NodeArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'node' -Arguments $NodeArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realNode = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    & $realNode @NodeArguments
    return $LASTEXITCODE
}

function vitest {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$VitestArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'vitest' -Arguments $VitestArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realVitest = Resolve-GloryExternalCommand -Name 'vitest'
    & $realVitest @VitestArguments
    return $LASTEXITCODE
}

function tsc {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$TscArguments)
    $qualityExit = Invoke-GloryQualityCommandGuard -Executable 'tsc' -Arguments $TscArguments
    if ($qualityExit -ne 0) { return $qualityExit }
    $realTsc = Resolve-GloryExternalCommand -Name 'tsc'
    & $realTsc @TscArguments
    return $LASTEXITCODE
}

function Get-GloryQualityGuardStatus {
    $qualityRoot = Find-GloryQualityRoot
    if (-not $qualityRoot) { Write-Host '[glory-quality] No hay quality.config.json en la ruta actual.'; return }
    $node = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    & $node (Join-Path $qualityRoot 'scripts\quality\heavy-run-guard.mjs') --status
}

Set-Alias glory-quality-status Get-GloryQualityGuardStatus -ErrorAction SilentlyContinue
