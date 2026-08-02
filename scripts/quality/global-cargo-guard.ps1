<#
.SYNOPSIS
    PowerShell interceptor for expensive Cargo commands.

.DESCRIPTION
    The project quality gate is still the source of truth. This function only
    intercepts raw `cargo test`, `cargo clippy` and `cargo bench` commands so an
    agent cannot bypass the three-hour cooldown by ignoring AGENTS.md.
    Non-Glory projects and light Cargo commands pass through unchanged.
#>

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

function cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments)
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

function Get-GloryQualityGuardStatus {
    $qualityRoot = Find-GloryQualityRoot
    if (-not $qualityRoot) { Write-Host '[glory-quality] No hay quality.config.json en la ruta actual.'; return }
    $node = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    & $node (Join-Path $qualityRoot 'scripts\quality\heavy-run-guard.mjs') --status
}

Set-Alias glory-quality-status Get-GloryQualityGuardStatus -ErrorAction SilentlyContinue
